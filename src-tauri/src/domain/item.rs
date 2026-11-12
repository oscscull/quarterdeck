use chrono::{DateTime, Utc};

use super::life::Life;
use super::turn::Turn;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Where a piece of work is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    UpNext,
    InProgress,
    InReview,
    Done,
}

/// What a piece of work is waiting on, shown as a chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Review,
    /// CI is running.
    Ci,
    /// CI failed: an agent should be fixing it.
    CiFailed,
    Dependency,
    /// Approved with CI passing: the user can merge it.
    Approved,
    Queued,
}

/// One thing in play: a ticket, PR or personal task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItem {
    pub id: String,
    pub title: String,
    pub state_label: String,
    /// Its stage, and whose move it is while it's active.
    #[serde(flatten)]
    pub life: Life,
    pub status: Option<Status>,
    pub area: String,
    /// Where the work lives: a ticket, PR or doc.
    pub link: Option<String>,
    /// `manual`, or the connector that made it, e.g. `github`.
    pub source: String,
    /// The connector's own id, e.g. `owner/repo#12`.
    pub external_id: Option<String>,
    pub opened_at: DateTime<Utc>,
    /// When the stage, status or label last changed.
    pub state_since: DateTime<Utc>,
    pub rank: i64,
}

impl WorkItem {
    pub const fn stage(&self) -> Stage {
        self.life.stage()
    }

    pub const fn turn(&self) -> Option<Turn> {
        self.life.turn()
    }

    pub fn is_turn(&self, turn: Turn) -> bool {
        self.turn() == Some(turn)
    }
}

/// The editable parts of an item, as sent from the UI or an agent. An empty `id` creates one.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemInput {
    pub id: String,
    pub title: String,
    pub state_label: String,
    pub stage: Stage,
    pub status: Option<Status>,
    pub area: String,
    pub link: Option<String>,
}

#[cfg(test)]
impl WorkItem {
    /// An item for tests, with `life` and everything else blank.
    pub fn sample(id: &str, life: Life) -> Self {
        let now = Utc::now();
        Self {
            id: id.to_owned(),
            title: id.to_owned(),
            state_label: String::new(),
            life,
            status: None,
            area: String::new(),
            link: None,
            source: String::new(),
            external_id: None,
            opened_at: now,
            state_since: now,
            rank: 0,
        }
    }
}
