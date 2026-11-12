//! What the agent is doing on an item: running, about to get an answer or an event's run, working
//! in a session outside Quarterdeck, or nothing.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};

use crate::config::turns::TurnSettings;
use crate::domain::event::Change;
use crate::domain::item::WorkItem;
use crate::domain::session::SessionRef;
use crate::domain::turn::Turn;
use crate::error::AppResult;
use crate::store::{Db, asks, events, fired, runs, watch};
use crate::turns::due::Work;

/// Unhandled events looked through for items about to get a run from one.
const EVENTS: u32 = 100;

/// What Quarterdeck knows of the agent's work across items, read once per tick.
pub struct Known {
    running: HashSet<String>,
    pending: HashSet<String>,
    sessions: HashMap<String, SessionRef>,
}

/// Items an answer is waiting to go to, or an event not yet handled is about: either may start
/// a run of its own (a CI fix, say), so prompting the agent too would run it twice.
async fn pending(db: &Db) -> AppResult<HashSet<String>> {
    let answered = asks::answered(db).await?.into_iter().map(|a| a.ask.item_id);
    let events = fired::new_events(db, EVENTS).await?.into_iter();
    Ok(answered.chain(events.map(|e| e.subject)).collect())
}

pub async fn known(db: &Db) -> AppResult<Known> {
    Ok(Known {
        running: runs::live(db)
            .await?
            .into_iter()
            .map(|r| r.target)
            .collect(),
        pending: pending(db).await?,
        sessions: watch::all(db).await?,
    })
}

/// When anything other than a turn change was last heard about the item.
async fn last_heard(db: &Db, item_id: &str) -> AppResult<Option<DateTime<Utc>>> {
    let history = events::for_item(db, item_id).await?;
    Ok(history
        .iter()
        .find(|e| !matches!(e.change, Change::Turn { .. }))
        .map(|e| e.at))
}

/// A session outside Quarterdeck may be working on the item: it's idle once nothing has been heard
/// since Quarterdeck's own last run ended, or nothing for `quiet` minutes.
async fn idle(db: &Db, item: &WorkItem, known: &Known, quiet: Duration) -> AppResult<bool> {
    if !known.sessions.contains_key(&item.id) {
        return Ok(true);
    }
    let Some(heard) = last_heard(db, &item.id).await? else {
        return Ok(true);
    };
    let ran = runs::last_finished_for_item(db, &item.id).await?;
    Ok(ran.is_some_and(|r| r >= heard) || Utc::now() - heard >= quiet)
}

pub async fn work(
    db: &Db,
    item: &WorkItem,
    known: &Known,
    limits: &TurnSettings,
) -> AppResult<Work> {
    if known.running.contains(&item.id) {
        return Ok(Work::Running);
    }
    if known.pending.contains(&item.id) {
        return Ok(Work::Pending);
    }
    let quiet = limits.quiet()?;
    if item.is_turn(Turn::Agent) && !idle(db, item, known, quiet).await? {
        return Ok(Work::Elsewhere);
    }
    Ok(Work::Idle)
}
