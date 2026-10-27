//! CI results go to whoever is on the work, so no agent ever polls CI. A Claude session that
//! claimed the item is resumed with the result. A failure nobody is on is handed to a new Claude
//! session, in the repo's project when there is one, and claimed so later results reach it too.

use tauri::{AppHandle, Manager as _};

use super::dispatch::{self, Job};
use super::thread::{self, Thread};
use crate::app::App;
use crate::config::prompts::{Prompt, render};
use crate::config::settings::Settings;
use crate::domain::fired::Fired;
use crate::domain::item::WorkItem;
use crate::domain::schedule::EventKind;
use crate::error::AppResult;
use crate::store::{items, watch};
use crate::workspace::{self, Workspace};

/// True for the events this reacts to.
pub fn is_ci(event: &Fired) -> bool {
    [EventKind::CiFailed, EventKind::CiPassed]
        .iter()
        .any(|k| k.name() == event.kind)
}

/// Reacts to a CI event. `covered` is true when a schedule already handles it.
pub async fn react(
    handle: &AppHandle,
    settings: &Settings,
    event: &Fired,
    covered: bool,
) -> AppResult<()> {
    let app = handle.state::<App>();
    let item = items::get(&app.db, &event.subject).await?;
    let ws = workspace::for_item(settings, &item)?;
    let claimed = watch::all(&app.db).await?.remove(&item.id);
    let job = match claimed {
        Some(session) => {
            let passed = event.kind == EventKind::CiPassed.name();
            let result = if passed { "passed" } else { "failed" };
            let prompt = prompt_for(&app, Prompt::CiDone, &item, &[("result", result)])?;
            let ws = ws.filter(|w| w.holds(&session));
            Job::new("CI result", item.id, prompt, Thread::Resume(session)).in_workspace(ws)
        }
        None if event.kind == EventKind::CiFailed.name() && settings.github.fix_ci && !covered => {
            let session = ws
                .as_ref()
                .map_or_else(thread::fresh_session, Workspace::new_session);
            let place = workspace::describe(&app.config, ws.as_ref())?;
            let prompt = prompt_for(&app, Prompt::FixCi, &item, &[("workspace", place.as_str())])?;
            Job::new("Fix CI", item.id, prompt, Thread::Start(session)).in_workspace(ws)
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

/// `extra` adds `result` and `workspace` to the item's own fields.
pub(super) fn prompt_for(
    app: &App,
    prompt: Prompt,
    item: &WorkItem,
    extra: &[(&str, &str)],
) -> AppResult<String> {
    let link = item.link.clone().unwrap_or_default();
    let mut vars = vec![
        ("item", item.id.as_str()),
        ("title", item.title.as_str()),
        ("link", link.as_str()),
    ];
    vars.extend_from_slice(extra);
    render(&app.config, prompt, &vars)
}
