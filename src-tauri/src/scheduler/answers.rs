//! Takes the user's answers back to Claude. Work on a PR, issue or repo in a known project happens in
//! that project's worktree: the asking session is resumed if it ran there, otherwise a new session
//! starts there with the question and answer. Elsewhere the session that asked, or the one holding
//! the item, is resumed, and with neither a new session starts in Quarterdeck's folder, told why
//! it has no checkout so it can fix the item's link or ask. An answer stays until a run reaches the
//! agent with it, so one sent while the agent is down goes again when it's back.

use std::collections::HashSet;

use tauri::{AppHandle, Manager as _};

use super::dispatch::{self, Job};
use super::failed::{self, Failure};
use super::thread::{self, Thread};
use crate::app::App;
use crate::claude::{reach, sessions};
use crate::config::prompts::{Prompt, render};
use crate::config::settings::Settings;
use crate::domain::ask::Answered;
use crate::domain::item::WorkItem;
use crate::domain::session::SessionRef;
use crate::error::AppResult;
use crate::store::{asks, items, runs};
use crate::workspace::{self, Located, Missing, Workspace};

pub async fn deliver(handle: &AppHandle, settings: &Settings) -> AppResult<()> {
    let app = handle.state::<App>();
    let waiting = asks::answered(&app.db).await?;
    // One run per item at a time: a second answer waits for the first run, then resumes its session.
    let mut busy: HashSet<String> = runs::live(&app.db)
        .await?
        .into_iter()
        .map(|r| r.target)
        .collect();
    for answered in waiting {
        if busy.contains(&answered.ask.item_id) {
            continue;
        }
        let item = items::get(&app.db, &answered.ask.item_id).await?;
        // Another machine runs this area's agents and delivers there.
        if settings.node_for(&item.area) != settings.node {
            continue;
        }
        match deliver_one(handle, settings, &answered, &item).await {
            // The run removes the answer once it reaches the agent.
            Ok(()) => {}
            Err(e) => not_started(handle, settings, (&answered, &item), &e.to_string()).await?,
        }
        busy.insert(item.id.clone());
    }
    Ok(())
}

/// A run that couldn't reach the agent keeps the answer for when it's back; any other failure is
/// reported and the answer dropped.
async fn not_started(
    handle: &AppHandle,
    settings: &Settings,
    (answered, item): (&Answered, &WorkItem),
    detail: &str,
) -> AppResult<()> {
    if reach::unreached(detail) {
        return super::outage::report(handle, settings, detail).await;
    }
    let failure = Failure {
        item_id: &item.id,
        what: "Answer",
        detail,
        run: None,
    };
    // An answer that will be tried again stays to go with it.
    match failed::report(handle, &failure, &settings.turns).await? {
        failed::Handled::TryingAgain => Ok(()),
        failed::Handled::HandedBack => {
            asks::remove(&handle.state::<App>().db, answered.ask.id).await
        }
    }
}

/// Starts the run that takes one answer to the agent.
async fn deliver_one(
    handle: &AppHandle,
    settings: &Settings,
    answered: &Answered,
    item: &WorkItem,
) -> AppResult<()> {
    let app = handle.state::<App>();
    let known = known_session(&app, answered, item).await?;
    let located = workspace::locate(settings, item)?;
    let thread = thread::thread_for(known, located.as_ref().ok());
    start(handle, settings, (answered, item), (thread, located)).await
}

/// Hands the answer to the agent.
async fn start(
    handle: &AppHandle,
    settings: &Settings,
    (answered, item): (&Answered, &WorkItem),
    (thread, located): (Thread, Located),
) -> AppResult<()> {
    let app = handle.state::<App>();
    let prompt = prompt_for(&app, answered, item, located.as_ref())?;
    let job = Job::new("Answer", item.id.clone(), prompt, thread)
        .in_workspace(located.ok())
        .delivering(answered.ask.id);
    dispatch::run(handle, settings, job).await
}

/// The session that asked, else the one holding the item, if Claude can resume it.
async fn known_session(
    app: &App,
    answered: &Answered,
    item: &WorkItem,
) -> AppResult<Option<SessionRef>> {
    if let Some(asker) = answered.ask.asker.clone().filter(sessions::exists) {
        return Ok(Some(asker));
    }
    super::held::session(&app.db, &item.id).await
}

fn prompt_for(
    app: &App,
    answered: &Answered,
    item: &WorkItem,
    located: Result<&Workspace, &Missing>,
) -> AppResult<String> {
    let link = item.link.clone().unwrap_or_default();
    let place = workspace::describe(&app.config, located)?;
    let vars = [
        ("item", item.id.as_str()),
        ("title", item.title.as_str()),
        ("link", link.as_str()),
        ("question", answered.ask.question.as_str()),
        ("context", answered.ask.context.as_str()),
        ("answer", answered.answer.as_str()),
        ("workspace", place.as_str()),
    ];
    let prompt = if answered.ask.question.is_empty() {
        Prompt::Message
    } else {
        Prompt::Answer
    };
    render(&app.config, prompt, &vars)
}
