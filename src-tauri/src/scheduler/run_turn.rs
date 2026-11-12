//! A run about an item happens on the agent's turn, and ends with the turn handed on: by the agent
//! through `pass_turn`, or back to the user with what the run said.

use chrono::{DateTime, Utc};
use tauri::{AppHandle, Manager as _};

use super::failed::{self, Failure};
use crate::app::App;
use crate::claude::reach;

use crate::config::turns::TurnSettings;
use crate::domain::turn::Turn;
use crate::error::AppResult;
use crate::store::{Db, items, watch};
use crate::turns::{self, Clock, Pass};

/// Gives the agent the turn, unless it has it already.
pub async fn take(db: &Db, (item_id, what): (&str, &str), limits: &TurnSettings) -> AppResult<()> {
    if items::get(db, item_id).await?.is_turn(Turn::Agent) {
        return Ok(());
    }
    turns::pass(db, item_id, Pass::to(Turn::Agent, what), Clock::now(limits)).await
}

/// A run about an item that finished: when it started, and what it said last.
#[derive(Debug, Clone, Copy)]
pub struct Finished<'a> {
    pub item_id: &'a str,
    pub started: DateTime<Utc>,
    pub said: &'a str,
}

/// After a run that finished: if the agent didn't hand the turn on, it goes to the user.
pub async fn end(db: &Db, run: Finished<'_>, limits: &TurnSettings) -> AppResult<()> {
    let item_id = run.item_id;
    let still_agents = items::get(db, item_id).await?.is_turn(Turn::Agent);
    if !still_agents || turns::history::passed_since(db, item_id, run.started).await? {
        return Ok(());
    }
    let why = format!("the agent stopped without handing over: {}", run.said);
    turns::pass(db, item_id, Pass::to(Turn::You, why), Clock::now(limits)).await
}

/// What came of a run about an item, once it ended.
pub struct Ended<'a> {
    pub item_id: &'a str,
    /// What the run was for, e.g. "Answer".
    pub what: &'a str,
    pub summary: &'a str,
    pub started: DateTime<Utc>,
    /// False on a build that may not run the agent, whose runs fail by design.
    pub may_run: bool,
    pub run: i64,
    /// Whether the run started a session, which won't exist if it never reached the agent.
    pub new_session: bool,
}

/// Hands the turn on after a run: through `end` when it finished, back to the user when the build
/// may not run the agent, kept by the agent when it couldn't reach it (see `outage`), and loudly
/// through `failed` when it failed.
pub async fn settle(handle: &AppHandle, ended: &Ended<'_>, limits: &TurnSettings) -> AppResult<()> {
    let db = &handle.state::<App>().db;
    let Some(detail) = ended.summary.strip_prefix("failed: ") else {
        let run = Finished {
            item_id: ended.item_id,
            started: ended.started,
            said: ended.summary,
        };
        return end(db, run, limits).await;
    };
    if !ended.may_run {
        let pass = Pass::to(Turn::You, detail);
        return turns::pass(db, ended.item_id, pass, Clock::now(limits)).await;
    }
    if reach::unreached(detail) {
        return wait_for_agent(handle, ended, detail).await;
    }
    let failure = Failure {
        item_id: ended.item_id,
        what: ended.what,
        detail,
        run: Some(ended.run),
    };
    failed::report(handle, &failure, limits).await.map(drop)
}

/// The item keeps the agent's turn, to be prompted again once the agent is back.
async fn wait_for_agent(handle: &AppHandle, ended: &Ended<'_>, detail: &str) -> AppResult<()> {
    let app = handle.state::<App>();
    if ended.new_session {
        watch::release(&app.db, ended.item_id).await?;
    }
    super::outage::report(handle, &app.settings()?, detail).await
}
