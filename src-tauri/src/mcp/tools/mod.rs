//! Every tool the MCP server offers, grouped by what it acts on.
#![allow(
    clippy::needless_pass_by_value,
    reason = "every handler has the same shape, (Ctx or &Ctx, Args), so both arrive by value"
)]

mod approve;
mod asks;
mod commitments;
mod events;
mod hello;
mod items;
mod outage;
mod overview;
mod projects;
mod queue;
mod runs;
mod schedules;
mod setting;
mod settings;
mod sync;
mod todos;
mod turns;
mod updates;
mod urgent;
mod watch;

use chrono::{DateTime, NaiveDate, Utc};

use super::tool::Tool;
use crate::error::{AppError, AppResult};

/// Tools only peers get: the user's own app on another machine, never an agent.
pub fn peer_only() -> Vec<Tool> {
    approve::tools().into_iter().chain(hello::tools()).collect()
}

pub fn all() -> Vec<Tool> {
    [
        overview::tools(),
        items::tools(),
        asks::tools(),
        todos::tools(),
        commitments::tools(),
        urgent::tools(),
        outage::tools(),
        updates::tools(),
        schedules::tools(),
        events::tools(),
        runs::tools(),
        settings::tools(),
        setting::tools(),
        sync::tools(),
        watch::tools(),
        turns::tools(),
        projects::tools(),
        queue::tools(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Parses an RFC 3339 time such as `2026-10-23T18:00:00+01:00`.
fn time(text: &str) -> AppResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(text)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| AppError::BadInput(format!("'{text}' is not an RFC 3339 time: {e}")))
}

/// Parses a calendar date such as `2026-10-23`.
fn date(text: &str) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .map_err(|e| AppError::BadInput(format!("'{text}' is not a YYYY-MM-DD date: {e}")))
}

/// Keeps things in `area`, or everything when no area is given.
fn in_area(area: Option<&String>, thing_area: &str) -> bool {
    area.is_none_or(|a| a == thing_area)
}
