//! Events waiting for the scheduler, and the latest ones it handled.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use super::client::{self, FiredRow};
use super::convert::{int, key};
use super::db::Db;
use super::time;
use crate::domain::fired::Fired;
use crate::error::{AppError, AppResult};

/// Set on the Quarterdeck MCP server a run starts, so events its agent causes count as one step
/// further down the chain than the event that started the run.
pub const DEPTH_VAR: &str = "QUARTERDECK_EVENT_DEPTH";

fn inherited_depth() -> u32 {
    std::env::var(DEPTH_VAR)
        .ok()
        .and_then(|d| d.parse().ok())
        .unwrap_or_default()
}

fn from_row(row: FiredRow) -> AppResult<Fired> {
    let data: BTreeMap<String, String> = serde_json::from_value(row.data)?;
    let started: Vec<String> = row
        .started
        .map(serde_json::from_value)
        .transpose()?
        .unwrap_or_default();
    Ok(Fired {
        id: int(&row.id)?,
        kind: row.kind,
        area: row.area,
        subject: row.subject,
        data,
        about: row.about,
        depth: u32::try_from(row.depth).map_err(|e| AppError::Db(e.to_string()))?,
        at: time::get(&row.at)?,
        started,
    })
}

fn from_rows(rows: Vec<FiredRow>) -> AppResult<Vec<Fired>> {
    rows.into_iter().map(from_row).collect()
}

/// Stores an event for the scheduler's next tick.
pub async fn emit(db: &Db, event: &Fired) -> AppResult<()> {
    let input = client::FireEventInput {
        kind: event.kind.clone(),
        area: event.area.clone(),
        subject: event.subject.clone(),
        data: serde_json::to_value(&event.data)?,
        about: event.about.clone(),
        depth: i64::from(event.depth.max(inherited_depth())),
        at: time::put(event.at),
    };
    Ok(db.api().fire_event(input, ()).await?)
}

/// Events not yet handled, oldest first.
pub async fn new_events(db: &Db, limit: u32) -> AppResult<Vec<Fired>> {
    let input = client::NewEventsInput {
        limit: i64::from(limit),
    };
    from_rows(db.api().new_events(input, ()).await?)
}

/// Marks an event handled, with the schedules it started.
pub async fn handled(db: &Db, id: i64, started: &[String]) -> AppResult<()> {
    let input = client::HandleEventInput {
        id: key(id),
        started: serde_json::to_value(started)?,
    };
    Ok(db.api().handle_event(input, ()).await?)
}

/// The latest handled events, newest first.
pub async fn recent(db: &Db, limit: u32) -> AppResult<Vec<Fired>> {
    let input = client::RecentEventsInput {
        limit: i64::from(limit),
    };
    from_rows(db.api().recent_events(input, ()).await?)
}

pub async fn expire(db: &Db, before: DateTime<Utc>) -> AppResult<()> {
    let input = client::ExpireEventsInput {
        before: time::put(before),
    };
    Ok(db.api().expire_events(input, ()).await?)
}
