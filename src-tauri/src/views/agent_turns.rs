//! The items that are the agent's turn, and whether a run or session holds each.

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::config::turns::TurnSettings;
use crate::domain::item::WorkItem;
use crate::domain::turn::Turn;
use crate::error::AppResult;
use crate::store::{Db, items};
use crate::turns::due::Work;
use crate::turns::work;

/// Where the agent's turn on an item stands.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Doing {
    /// A Quarterdeck run is working on it.
    Running,
    /// A session outside Quarterdeck was heard from lately.
    Session,
    /// Quarterdeck prompts it, or delivers its answer, on the next tick.
    Starting,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTurn {
    pub item_id: String,
    pub title: String,
    pub area: String,
    pub doing: Doing,
    /// When the agent's turn runs out and passes back to the user.
    pub until: Option<DateTime<Utc>>,
}

fn agents(item: &WorkItem) -> bool {
    item.is_turn(Turn::Agent)
}

const fn doing(work: Work) -> Doing {
    match work {
        Work::Running => Doing::Running,
        Work::Elsewhere => Doing::Session,
        Work::Pending | Work::Idle => Doing::Starting,
    }
}

/// Classified as the keeper sees them, so this list and the keeper never disagree.
pub async fn build(db: &Db, limits: &TurnSettings) -> AppResult<Vec<AgentTurn>> {
    let known = work::known(db).await?;
    let mut turns = Vec::new();
    for i in items::list(db).await?.into_iter().filter(agents) {
        let doing = doing(work::work(db, &i, &known, limits).await?);
        turns.push(AgentTurn {
            item_id: i.id,
            title: i.title,
            area: i.area,
            doing,
            until: i.life.until(),
        });
    }
    Ok(turns)
}
