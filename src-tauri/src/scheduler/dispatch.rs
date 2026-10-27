//! Starts one Claude run about an item and records it on the Agent tab while it works.

use chrono::{DateTime, Utc};
use tauri::{AppHandle, Manager as _};

use super::run_turn;
use super::thread::Thread;
use crate::app::App;
use crate::claude::{self, Ask, Session, reach};
use crate::config::Config;
use crate::config::prompts::{Prompt, render};
use crate::config::settings::Settings;
use crate::config::turns::TurnSettings;
use crate::domain::run::{Outcome, Run};
use crate::error::AppResult;
use crate::store::{asks, run_prompts, runs, watch};
use crate::workspace::{self, Kind, Workspace};

#[derive(Debug, Clone)]
pub struct Job {
    /// Shown on the Agent tab, e.g. "Fix CI".
    pub what: String,
    pub item_id: String,
    pub prompt: String,
    pub thread: Thread,
    /// Claude Code tools it may use besides the Quarterdeck tools.
    pub tools: Vec<String>,
    /// The worktree to make first, for work in a project. Edits are allowed there.
    pub workspace: Option<Workspace>,
    /// The answered question this run delivers. It's removed once the run reached the agent, so an
    /// answer the agent never got goes again when it's back.
    pub ask: Option<i64>,
}

impl Job {
    pub fn new(what: &str, item_id: String, prompt: String, thread: Thread) -> Self {
        Self {
            what: what.to_owned(),
            item_id,
            prompt,
            thread,
            tools: Vec::new(),
            workspace: None,
            ask: None,
        }
    }

    pub fn delivering(self, ask: i64) -> Self {
        Self {
            ask: Some(ask),
            ..self
        }
    }

    pub fn with_tools(self, tools: &[String]) -> Self {
        Self {
            tools: tools.to_vec(),
            ..self
        }
    }

    pub fn in_workspace(self, workspace: Option<Workspace>) -> Self {
        Self { workspace, ..self }
    }
}

async fn start_record(
    handle: &AppHandle,
    settings: &Settings,
    job: &Job,
    started: DateTime<Utc>,
) -> AppResult<i64> {
    let record = Run {
        id: 0,
        schedule_id: None,
        what: job.what.clone(),
        target: job.item_id.clone(),
        node: settings.node.clone(),
        started_at: started,
        finished_at: None,
        outcome: None,
        summary: None,
    };
    runs::start(&handle.state::<App>().db, &record).await
}

fn first_line(text: &str) -> String {
    text.lines()
        .next()
        .unwrap_or_default()
        .chars()
        .take(200)
        .collect()
}

/// A new session on an issue starts with the issue's text, which Claude may not be able to fetch.
async fn prompt_for(job: &Job) -> AppResult<String> {
    match (&job.workspace, &job.thread) {
        (
            Some(
                ws @ Workspace {
                    kind: Kind::Issue(number),
                    ..
                },
            ),
            Thread::Start(_),
        ) => {
            let issue = workspace::issue_text(ws, *number).await?;
            Ok(format!("{}\n\n---\n\n{issue}", job.prompt))
        }
        _ => Ok(job.prompt.clone()),
    }
}

/// How to run a job: `mode` applies to work in a project's worktree, `run` is its record.
struct How {
    /// An error when this build may not run Claude; the run records it as failed.
    command: AppResult<String>,
    mode: String,
    run: i64,
    started: DateTime<Utc>,
    /// The turn limits as the run started, so a config file mid-edit can't stop its hand-over.
    limits: TurnSettings,
}

async fn ask(how: &How, job: &Job) -> String {
    let command = match &how.command {
        Ok(c) => c,
        Err(e) => return format!("failed: {e}"),
    };
    if let Some(ws) = &job.workspace
        && let Err(e) = workspace::prepare(ws).await
    {
        return format!("failed: couldn't make the worktree: {e}");
    }
    let prompt = match prompt_for(job).await {
        Ok(p) => p,
        Err(e) => return format!("failed: couldn't read the issue: {e}"),
    };
    let s = job.thread.session();
    let session = match job.thread {
        Thread::Start(_) => Session::Start(&s.id),
        Thread::Resume(_) => Session::Resume(&s.id),
    };
    let opts = Ask {
        tools: &job.tools,
        session,
        cwd: s.cwd.as_deref(),
        permission_mode: job.workspace.as_ref().map(|_| how.mode.as_str()),
        run: Some(how.run),
        depth: 0,
    };
    match claude::ask_with(command, &prompt, &opts).await {
        Ok(text) => first_line(&text),
        Err(e) => format!("failed: {e}"),
    }
}

/// Runs the job to its end, then records it and settles the item's turn.
async fn work(handle: AppHandle, job: Job, how: How) {
    let summary = ask(&how, &job).await;
    let failure = summary.strip_prefix("failed: ");
    let outcome = if failure.is_some() {
        Outcome::Failed
    } else {
        Outcome::Ok
    };
    // Before the run is marked finished, so the answer can't be picked up and sent twice. If this
    // fails the answer goes again, which beats losing it.
    if let Some(ask) = job.ask
        && !failure.is_some_and(reach::unreached)
    {
        drop(asks::remove(&handle.state::<App>().db, ask).await);
    }
    if outcome == Outcome::Ok
        && let Some(ws) = &job.workspace
    {
        super::handoff::after_run(&handle, (&job.item_id, ws), &how.limits).await;
    }
    // The run record is for display; losing it changes nothing about the item.
    drop(runs::finish(&handle.state::<App>().db, how.run, outcome, &summary).await);
    let ended = run_turn::Ended {
        item_id: &job.item_id,
        what: &job.what,
        summary: &summary,
        started: how.started,
        may_run: how.command.is_ok(),
        run: how.run,
        new_session: matches!(job.thread, Thread::Start(_)),
    };
    // Nothing is left to tell if even this fails; the run record above still shows it.
    drop(run_turn::settle(&handle, &ended, &how.limits).await);
}

/// The job's prompt with the rule for handing the turn on after it.
fn with_hand_over(config: &Config, job: Job) -> AppResult<Job> {
    let rule = render(config, Prompt::HandOver, &[("item", job.item_id.as_str())])?;
    let prompt = format!("{}\n\n{rule}", job.prompt);
    Ok(Job { prompt, ..job })
}

pub async fn run(handle: &AppHandle, settings: &Settings, job: Job) -> AppResult<()> {
    let app = handle.state::<App>();
    let asked = job.prompt.clone();
    let job = with_hand_over(&app.config, job)?;
    run_turn::take(&app.db, (&job.item_id, &job.what), &settings.turns).await?;
    let started = Utc::now();
    let how = How {
        command: settings.claude().map(str::to_owned),
        mode: settings.project_permission_mode.clone(),
        run: start_record(handle, settings, &job, started).await?,
        started,
        limits: settings.turns.clone(),
    };
    // Kept only so a failed run can be retried; the run goes ahead without it.
    drop(run_prompts::keep(&app.db, how.run, &asked).await);
    // A new session is the item's from now on, so later answers and CI results resume it.
    if let Thread::Start(session) = &job.thread
        && let Err(e) = watch::claim(&app.db, &job.item_id, session).await
    {
        let said = format!("failed: couldn't record the session: {e}");
        drop(runs::finish(&app.db, how.run, Outcome::Failed, &said).await);
        return Err(e);
    }
    tauri::async_runtime::spawn(work(handle.clone(), job, how));
    Ok(())
}
