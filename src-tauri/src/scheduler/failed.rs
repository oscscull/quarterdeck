//! An agent run about an item that failed: tried again by itself while it has retries left (see
//! `again`), then handed back to the user.

use chrono::Utc;
use tauri::{AppHandle, Manager as _};

use crate::app::App;
use crate::config::turns::TurnSettings;
use crate::domain::item::WorkItem;
use crate::domain::turn::Turn;
use crate::domain::urgent::Urgent;
use crate::error::AppResult;
use crate::notify;
use crate::store::{Db, items, urgent, watch};
use crate::turns::{self, Clock, Pass};

pub struct Failure<'a> {
    pub item_id: &'a str,
    /// What the run was for, e.g. "Answer".
    pub what: &'a str,
    pub detail: &'a str,
    /// The run that failed, which the alert offers to retry. None when it never started.
    pub run: Option<i64>,
}

/// Starts the need of every item handed back because its run failed.
pub const COULDNT_RUN: &str = "your agent couldn't run";

/// The title of the alert raised about `item_id`'s failed run.
pub fn alert_title(item_id: &str) -> String {
    format!("Your agent couldn't run on {item_id}")
}

/// What to show when a run died without saying why.
const NO_DETAIL: &str = "it stopped without saying why";
const NEXT_STEP: &str = "Open its run on the Agent tab, or tell Claude to look into it.";
const NEXT_STEP_RETRY: &str = "Retry it if it may have been a blip, open its run on the Agent tab, or tell Claude to look into it.";
/// The longest reason an alert shows; the item's history keeps all of it.
const BRIEF_CHARS: usize = 200;

impl Failure<'_> {
    pub fn detail(&self) -> &str {
        if self.detail.trim().is_empty() {
            NO_DETAIL
        } else {
            self.detail
        }
    }

    /// The first line of the reason and what to do, for an alert.
    fn brief(&self) -> String {
        let first: String = self
            .detail()
            .lines()
            .next()
            .unwrap_or_default()
            .chars()
            .take(BRIEF_CHARS)
            .collect();
        let next = if self.run.is_some() {
            NEXT_STEP_RETRY
        } else {
            NEXT_STEP
        };
        format!("{first}. {next}")
    }
}

async fn raise_urgent(db: &Db, item: &WorkItem, f: &Failure<'_>) -> AppResult<String> {
    let title = alert_title(&item.id);
    let raised = Urgent {
        id: 0,
        title: title.clone(),
        detail: f.brief(),
        area: item.area.clone(),
        raised_at: Utc::now(),
        schedule_id: None,
        run_id: f.run,
    };
    urgent::raise(db, &raised).await?;
    Ok(title)
}

async fn hand_back(db: &Db, f: &Failure<'_>, limits: &TurnSettings) -> AppResult<String> {
    watch::release(db, f.item_id).await?;
    let item = items::get(db, f.item_id).await?;
    let why = format!("{COULDNT_RUN} ({}): {}", f.what, f.detail());
    turns::pass(db, &item.id, Pass::to(Turn::You, why), Clock::now(limits)).await?;
    raise_urgent(db, &item, f).await
}

/// What came of a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handled {
    /// The agent keeps the turn and tries again in a new session.
    TryingAgain,
    /// Its retries are used up: the user has it, with an urgent item.
    HandedBack,
}

pub async fn report(
    handle: &AppHandle,
    failure: &Failure<'_>,
    limits: &TurnSettings,
) -> AppResult<Handled> {
    let db = &handle.state::<App>().db;
    if super::again::try_again(db, failure, limits).await? {
        return Ok(Handled::TryingAgain);
    }
    let title = hand_back(db, failure, limits).await?;
    notify::send(handle, &title, &failure.brief())?;
    Ok(Handled::HandedBack)
}
