//! Prompting the agent on its turn when nothing is running: the session holding the item carries
//! on, or a new one starts in the item's worktree (or Quarterdeck's folder, with no project). A
//! retry asked for from an alert sends the failed run's prompt instead.

use tauri::{AppHandle, Manager as _};

use super::dispatch::{self, Job};
use super::thread::{self, Thread};
use crate::app::App;
use crate::config::prompts::{Prompt, render};
use crate::config::settings::Settings;
use crate::domain::item::WorkItem;
use crate::error::AppResult;
use crate::retry;
use crate::turns;
use crate::workspace::{self, Located, Missing, Workspace};

/// The session to carry on in, and the worktree it runs in.
async fn thread(app: &App, settings: &Settings, item: &WorkItem) -> AppResult<(Thread, Located)> {
    let located = workspace::locate(settings, item)?;
    let known = super::held::session(&app.db, &item.id).await?;
    let thread = thread::thread_for(known, located.as_ref().ok());
    Ok((thread, located))
}

async fn prompt_for(
    app: &App,
    item: &WorkItem,
    located: Result<&Workspace, &Missing>,
) -> AppResult<String> {
    let why = turns::history::last_why(&app.db, &item.id)
        .await?
        .unwrap_or_default();
    let link = item.link.clone().unwrap_or_default();
    let place = workspace::describe(&app.config, located)?;
    let vars = [
        ("item", item.id.as_str()),
        ("title", item.title.as_str()),
        ("link", link.as_str()),
        ("label", item.state_label.as_str()),
        ("why", why.as_str()),
        ("workspace", place.as_str()),
    ];
    render(&app.config, Prompt::Continue, &vars)
}

/// The failed run's prompt when a retry was asked for, otherwise "carry on".
async fn job_for(
    app: &App,
    item: &WorkItem,
    thread: Thread,
    located: Result<&Workspace, &Missing>,
) -> AppResult<Job> {
    if let Some(asked) = retry::take(&app.db, &item.id).await? {
        return Ok(Job::new(&asked.what, item.id.clone(), asked.prompt, thread));
    }
    let prompt = prompt_for(app, item, located).await?;
    Ok(Job::new("Carry on", item.id.clone(), prompt, thread))
}

pub async fn carry_on(handle: &AppHandle, settings: &Settings, item: &WorkItem) -> AppResult<()> {
    // A build that may not run the agent would only fail the same run every tick.
    if !settings.may_run_claude() {
        return Ok(());
    }
    let app = handle.state::<App>();
    let (thread, located) = thread(&app, settings, item).await?;
    let job = job_for(&app, item, thread, located.as_ref())
        .await?
        .in_workspace(located.ok());
    dispatch::run(handle, settings, job).await
}
