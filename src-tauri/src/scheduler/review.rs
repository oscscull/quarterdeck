//! Review comments go to Claude before the user: the session on the item is resumed with them, or,
//! if nobody is on it, a new one starts. Claude makes the changes it can and replies with the
//! commit, and hands the user only what needs their judgement (`prompts/address-review.md`).

use tauri::{AppHandle, Manager as _};

use super::ci::prompt_for;
use super::dispatch::{self, Job};
use super::thread::{self, Thread};
use crate::app::App;
use crate::config::prompts::Prompt;
use crate::config::settings::Settings;
use crate::domain::fired::Fired;
use crate::domain::schedule::EventKind;
use crate::error::AppResult;
use crate::store::items;
use crate::workspace::{self, Workspace};

/// True for the events this reacts to.
pub fn is_review(event: &Fired) -> bool {
    event.kind == EventKind::PrNewComments.name()
}

/// Reacts to new comments. `covered` is true when a schedule already handles them.
pub async fn react(
    handle: &AppHandle,
    settings: &Settings,
    event: &Fired,
    covered: bool,
) -> AppResult<()> {
    if covered || !settings.github.fixes.reviews {
        return Ok(());
    }
    let app = handle.state::<App>();
    let item = items::get(&app.db, &event.subject).await?;
    let located = workspace::locate(settings, &item)?;
    let place = workspace::describe(&app.config, located.as_ref())?;
    let ws = located.ok();
    let vars = [("workspace", place.as_str())];
    let prompt = prompt_for(&app, Prompt::AddressReview, &item, &vars)?;
    let (thread, ws) = if let Some(session) = super::held::session(&app.db, &item.id).await? {
        let ws = ws.filter(|w| w.holds(&session));
        (Thread::Resume(session), ws)
    } else {
        let session = ws
            .as_ref()
            .map_or_else(thread::fresh_session, Workspace::new_session);
        (Thread::Start(session), ws)
    };
    let job = Job::new("Review comments", item.id, prompt, thread).in_workspace(ws);
    dispatch::run(
        handle,
        settings,
        job.with_tools(&settings.github.fixes.tools),
    )
    .await
}
