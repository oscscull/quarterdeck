use chrono::Local;
use serde::Serialize;

use crate::config::pause::Pause;
use crate::config::schedules::Loaded;
use crate::config::turns::TurnSettings;
use crate::connectors::{calendar, github, status};
use crate::cron::explain::{Explanation, explain};
use crate::domain::fired::Fired;
use crate::domain::run::Run;
use crate::domain::schedule::Schedule;
use crate::error::AppResult;
use crate::jobs::approval;
use crate::store::{Db, fired, runs};
use crate::views::agent_turns::{self, AgentTurn};
use crate::views::live::{self, LiveRun};

const NEXT_RUNS: usize = 3;
const RECENT_RUNS: u32 = 3;
const RECENT_EVENTS: u32 = 20;

/// A schedule with when it runs and what it did lately.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleView {
    #[serde(flatten)]
    pub schedule: Schedule,
    /// Its first cron trigger explained; missing when it has none.
    pub explanation: Option<Explanation>,
    pub runs: Vec<Run>,
    /// A script waiting for the user to approve it as it stands.
    pub needs_approval: bool,
}

/// Everything the Agent screen shows.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentView {
    pub live: Vec<LiveRun>,
    /// Items that are the agent's turn, running or about to be.
    pub turns: Vec<AgentTurn>,
    pub schedules: Vec<ScheduleView>,
    /// The latest events, newest first, with the schedules each started.
    pub events: Vec<Fired>,
    pub problems: Vec<String>,
    pub pause: Option<Pause>,
    pub connections: Vec<status::Status>,
}

pub async fn build(
    db: &Db,
    loaded: Loaded,
    pause: Option<Pause>,
    limits: &TurnSettings,
) -> AppResult<AgentView> {
    let mut problems = loaded.problems;
    let mut schedules = Vec::with_capacity(loaded.schedules.len());
    for schedule in loaded.schedules {
        let mut explained = schedule.crons().map(|expr| {
            explain(expr, Local::now(), NEXT_RUNS)
                .map_err(|e| problems.push(format!("{}: {e}", schedule.id)))
                .ok()
        });
        let explanation = explained.next().flatten();
        explained.for_each(drop);
        let recent = runs::recent_for(db, &schedule.id, RECENT_RUNS).await?;
        let needs_approval = !approval::approved(db, &schedule).await?;
        schedules.push(ScheduleView {
            schedule,
            explanation,
            runs: recent,
            needs_approval,
        });
    }
    Ok(AgentView {
        live: live::build(db).await?,
        turns: agent_turns::build(db, limits).await?,
        schedules,
        events: fired::recent(db, RECENT_EVENTS).await?,
        problems,
        pause,
        connections: vec![
            status::read(db, github::SOURCE).await?,
            status::read(db, github::issues::SOURCE).await?,
            status::read(db, calendar::SOURCE).await?,
        ],
    })
}
