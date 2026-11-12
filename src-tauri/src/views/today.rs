use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::connectors::github::issues;
use crate::domain::activity::Activity;
use crate::domain::commitment::Commitment;
use crate::domain::item::{Stage, WorkItem};
use crate::domain::outage::Outage;
use crate::domain::todo::Todo;
use crate::domain::urgent::Urgent;
use crate::error::AppResult;
use crate::priorities::{Priority, priorities};
use crate::scheduler::outage;
use crate::store::{Db, activity, asks, commitments, items, runs, todos, urgent};
use crate::today::upcoming;

const ACTIVITY_SHOWN: u32 = 12;
/// Enough upcoming commitments for any one area to still show its next two.
const UPCOMING_SHOWN: usize = 12;

/// An issue nobody has picked up yet. Today shows how many, not each one.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Triage {
    pub item_id: String,
    pub title: String,
    pub area: String,
}

fn triage(items: &[WorkItem]) -> Vec<Triage> {
    items
        .iter()
        .filter(|i| issues::untriaged(i))
        .map(|i| Triage {
            item_id: i.id.clone(),
            title: i.title.clone(),
            area: i.area.clone(),
        })
        .collect()
}

/// Everything the Today screen shows.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayView {
    /// Set while the agent can't run at all; its work waits until it's back.
    pub outage: Option<Outage>,
    pub urgent: Vec<Urgent>,
    pub upcoming: Vec<Commitment>,
    /// Every ask in rank order; the UI shows the first `priority_limit` in the chosen area.
    pub priorities: Vec<Priority>,
    pub priority_limit: usize,
    pub todos: Vec<Todo>,
    pub triage: Vec<Triage>,
    pub activity: Vec<Activity>,
    pub running: usize,
    pub in_flight: usize,
}

/// What Today shows as of `now`.
pub async fn build(db: &Db, priority_limit: usize, now: DateTime<Utc>) -> AppResult<TodayView> {
    let ranked = items::list(db).await?;
    let in_flight = ranked
        .iter()
        .filter(|i| !matches!(i.stage(), Stage::UpNext | Stage::Done))
        .count();
    Ok(TodayView {
        outage: outage::current(db).await?,
        urgent: urgent::list(db).await?,
        upcoming: upcoming(commitments::list(db).await?, now)
            .into_iter()
            .take(UPCOMING_SHOWN)
            .collect(),
        priorities: priorities(&ranked, &asks::list(db).await?),
        priority_limit,
        todos: todos::list(db).await?,
        triage: triage(&ranked),
        activity: activity::recent(db, ACTIVITY_SHOWN).await?,
        running: runs::live(db).await?.len(),
        in_flight,
    })
}
