//! What the briefing is written from: one area's projects, its open and just-finished items with
//! their records, and its references.

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

use crate::domain::event::{Change, RecordTag};
use crate::domain::item::{Stage, WorkItem};
use crate::domain::life::LifeFields;
use crate::domain::project::{Briefing, Project};
use crate::domain::reference::Reference;
use crate::domain::turn::Turn;
use crate::error::AppResult;
use crate::store::{Db, events, items, projects, references};

/// How many of an item's records the writer sees, newest first.
const RECORDS_PER_ITEM: usize = 8;

/// One record line on an item. `tag` is `None` for notes kept from before records existed.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecordLine {
    pub id: i64,
    pub at: DateTime<Utc>,
    pub tag: Option<RecordTag>,
    pub text: String,
}

/// An item as the writer sees it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputItem {
    pub id: String,
    pub title: String,
    pub stage: Stage,
    pub turn: Option<Turn>,
    pub state_label: String,
    pub need: Option<String>,
    pub project: Option<String>,
    pub link: Option<String>,
    /// When it finished, for done items.
    pub done_at: Option<DateTime<Utc>>,
    pub records: Vec<RecordLine>,
}

/// Everything one area's briefing is written from.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inputs {
    pub area: String,
    pub now: DateTime<Utc>,
    /// Work that finished after this is "just done"; older finished work is left out.
    pub just_done_since: DateTime<Utc>,
    pub projects: Vec<Project>,
    pub items: Vec<InputItem>,
    pub references: Vec<Reference>,
    /// The area's combined briefing as last written, if any.
    pub current: Option<Briefing>,
}

impl Inputs {
    /// Nothing to write about: no projects and no work in the window.
    pub const fn is_empty(&self) -> bool {
        self.projects.is_empty() && self.items.is_empty()
    }
}

/// The record lines in an item's history, newest first.
fn records(history: Vec<crate::domain::event::ItemEvent>) -> Vec<RecordLine> {
    history
        .into_iter()
        .filter(|e| !e.trace)
        .filter_map(|e| {
            let (tag, text) = match e.change {
                Change::Record { tag, text } => (Some(tag), text),
                Change::Note { text } => (None, text),
                Change::Created
                | Change::Stage { .. }
                | Change::Status { .. }
                | Change::Label { .. }
                | Change::Turn { .. } => return None,
            };
            Some(RecordLine {
                id: e.id,
                at: e.at,
                tag,
                text,
            })
        })
        .take(RECORDS_PER_ITEM)
        .collect()
}

/// Whether an item belongs in the briefing: open, or finished inside the window.
fn in_window(item: &WorkItem, since: DateTime<Utc>) -> bool {
    item.stage() != Stage::Done || item.state_since >= since
}

async fn input_item(db: &Db, item: WorkItem, project: Option<String>) -> AppResult<InputItem> {
    let done = item.stage() == Stage::Done;
    let history = events::for_item(db, &item.id).await?;
    Ok(InputItem {
        stage: item.stage(),
        turn: item.turn(),
        need: LifeFields::from(item.life.clone()).need,
        done_at: done.then_some(item.state_since),
        records: records(history),
        id: item.id,
        title: item.title,
        state_label: item.state_label,
        project,
        link: item.link,
    })
}

/// Gathers `area`'s inputs as of `now`.
pub async fn gather(
    db: &Db,
    area: &str,
    just_done_days: i64,
    now: DateTime<Utc>,
) -> AppResult<Inputs> {
    let since = now - Duration::days(just_done_days);
    let membership = items::projects(db).await?;
    let mut list = Vec::new();
    for item in items::list(db).await? {
        if item.area == area && in_window(&item, since) {
            let project = membership.get(&item.id).cloned();
            list.push(input_item(db, item, project).await?);
        }
    }
    Ok(Inputs {
        area: area.to_owned(),
        now,
        just_done_since: since,
        projects: projects::in_area(db, area).await?,
        items: list,
        references: references::in_area(db, area).await?,
        current: projects::area_briefing(db, area).await?.map(|b| b.briefing),
    })
}
