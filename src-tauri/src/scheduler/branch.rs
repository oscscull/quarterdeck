//! Keeps PRs mergeable while they wait. A PR that fell behind a base it must be up to date with
//! has its branch updated here, with no Claude run. One that conflicts goes to the session on it,
//! or to a new one, like a CI failure.

use tauri::{AppHandle, Manager as _};

use super::ci::prompt_for;
use super::dispatch::{self, Job};
use super::thread::{self, Thread};
use crate::app::App;
use crate::config::prompts::Prompt;
use crate::config::settings::Settings;
use crate::connectors::github;
use crate::domain::event::Change;
use crate::domain::fired::Fired;
use crate::domain::schedule::EventKind;
use crate::domain::turn::Turn;
use crate::error::AppResult;
use crate::profile;
use crate::store::{Db, events, items, watch};
use crate::turns::{self, Clock, Pass};
use crate::workspace::{self, Workspace};

/// True for the events this reacts to.
pub fn is_branch(event: &Fired) -> bool {
    [EventKind::PrBehind, EventKind::PrConflicts]
        .iter()
        .any(|k| k.name() == event.kind)
}

/// Reacts to a branch event. `covered` is true when a schedule already handles it.
pub async fn react(
    handle: &AppHandle,
    settings: &Settings,
    event: &Fired,
    covered: bool,
) -> AppResult<()> {
    if covered {
        return Ok(());
    }
    if event.kind == EventKind::PrBehind.name() {
        return update(&handle.state::<App>().db, settings, event).await;
    }
    fix_conflicts(handle, settings, event).await
}

/// Updates the branch on GitHub. If GitHub refuses, the user hears why.
async fn update(db: &Db, settings: &Settings, event: &Fired) -> AppResult<()> {
    // A dev build's copied data points at real PRs; only change them when it may run Claude.
    if !settings.github.branches.update_behind || (profile::DEV && !settings.dev.run_claude) {
        return Ok(());
    }
    let url = event.data.get("url").map_or("", String::as_str);
    let Err(e) = github::update_branch(url).await else {
        return Ok(());
    };
    let now = chrono::Utc::now();
    let text = format!("Couldn't update the branch from its base: {e}");
    events::record(db, &event.subject, now, &Change::Note { text }).await?;
    let pass = Pass::to(Turn::You, "branch is behind its base")
        .needing("Update the branch: it's behind its base and can't merge until it is");
    turns::pass(db, &event.subject, pass, Clock::now(&settings.turns)).await
}

/// Resumes the session that claimed the item, else starts one, to resolve the conflicts.
async fn fix_conflicts(handle: &AppHandle, settings: &Settings, event: &Fired) -> AppResult<()> {
    let app = handle.state::<App>();
    let item = items::get(&app.db, &event.subject).await?;
    let ws = workspace::for_item(settings, &item)?;
    let place = workspace::describe(&app.config, ws.as_ref())?;
    let prompt = prompt_for(
        &app,
        Prompt::FixConflicts,
        &item,
        &[("workspace", place.as_str())],
    )?;
    let job = match watch::all(&app.db).await?.remove(&item.id) {
        Some(session) => {
            let ws = ws.filter(|w| w.holds(&session));
            Job::new("Fix conflicts", item.id, prompt, Thread::Resume(session)).in_workspace(ws)
        }
        None if settings.github.branches.fix_conflicts => {
            let session = ws
                .as_ref()
                .map_or_else(thread::fresh_session, Workspace::new_session);
            Job::new("Fix conflicts", item.id, prompt, Thread::Start(session)).in_workspace(ws)
        }
        None => return Ok(()),
    };
    dispatch::run(
        handle,
        settings,
        job.with_tools(&settings.github.fix_ci_tools),
    )
    .await
}
