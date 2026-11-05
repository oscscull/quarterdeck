//! Keeps every open item moving: each tick, an item whose clock calls for something gets it. The
//! agent is prompted on its turn, the user reminded on theirs, and a wait that runs out passes the
//! turn on. The rules are in `turns::due`; this carries them out.
//!
//! Clocks never stop. This machine keeps every item in its database, whatever area it's in, since
//! no other machine holds that record. While the agent can't be prompted (paused, or it can't run)
//! only prompting stops: its turns still run out to the user.

use chrono::Utc;
use tauri::{AppHandle, Manager as _};

use super::agent_prompt;
use super::failed::{self, Failure};
use crate::app::App;
use crate::claude::reach;
use crate::config::settings::Settings;
use crate::connectors::github::issues;
use crate::domain::item::WorkItem;
use crate::domain::turn::Turn;
use crate::error::AppResult;
use crate::notify;
use crate::store::items;
use crate::turns::due::{Due, due};
use crate::turns::work::{self, Known};
use crate::turns::{self, Clock, Pass};

async fn act(handle: &AppHandle, keeping: Keeping<'_>, item: &WorkItem, due: Due) -> AppResult<()> {
    let Keeping {
        settings,
        prompting,
    } = keeping;
    let db = &handle.state::<App>().db;
    let clock = Clock::now(&settings.turns);
    match due {
        Due::Wait => Ok(()),
        Due::Prompt if !prompting => Ok(()),
        Due::StartClock => {
            turns::reset_clock(
                db,
                &item.id,
                clock.deadline(item.turn().unwrap_or(Turn::You))?,
            )
            .await
        }
        Due::Prompt => prompt(handle, settings, item).await,
        Due::Remind => {
            turns::reset_clock(db, &item.id, clock.deadline(Turn::You)?).await?;
            // Issues to triage are counted on Today's triage line; one notice each a day is noise.
            if !issues::untriaged(item) {
                tell_user(handle, item, "Still your turn");
            }
            Ok(())
        }
        Due::PassTo { to, why } => {
            turns::pass(db, &item.id, Pass::to(to, why), clock).await?;
            if to == Turn::You {
                tell_user(handle, item, "Your turn");
            }
            Ok(())
        }
    }
}

/// A prompt that can't start is a failed run: the turn comes back with why, and loudly. One that
/// couldn't reach the agent waits for it instead.
async fn prompt(handle: &AppHandle, settings: &Settings, item: &WorkItem) -> AppResult<()> {
    let Err(e) = agent_prompt::carry_on(handle, settings, item).await else {
        return Ok(());
    };
    let detail = e.to_string();
    if reach::unreached(&detail) {
        return super::outage::report(handle, settings, &detail).await;
    }
    let failure = Failure {
        item_id: &item.id,
        what: "Carry on",
        detail: &detail,
        run: None,
    };
    failed::report(handle, &failure, &settings.turns)
        .await
        .map(drop)
}

/// A notification is a courtesy; the turn is shown in the app either way.
fn tell_user(handle: &AppHandle, item: &WorkItem, heading: &str) {
    let title = format!("{heading}: {}", item.title);
    drop(notify::send(handle, &title, &item.state_label));
}

/// The settings, and whether the agent may be prompted now.
#[derive(Clone, Copy)]
pub struct Keeping<'a> {
    pub settings: &'a Settings,
    pub prompting: bool,
}

async fn keep(
    handle: &AppHandle,
    keeping: Keeping<'_>,
    item: &WorkItem,
    known: &Known,
) -> AppResult<()> {
    let db = &handle.state::<App>().db;
    let limits = &keeping.settings.turns;
    let work = work::work(db, item, known, limits).await?;
    let due = due(item, work, limits, Utc::now());
    act(handle, keeping, item, due).await
}

/// Every item gets its turn kept, even when one fails; the first failure is reported.
pub async fn check(handle: &AppHandle, keeping: Keeping<'_>) -> AppResult<()> {
    let app = handle.state::<App>();
    let known = work::known(&app.db).await?;
    let mine = items::list(&app.db).await?;
    let mut first = Ok(());
    for item in &mine {
        let kept = keep(handle, keeping, item, &known).await;
        if first.is_ok() {
            first = kept;
        }
    }
    first
}
