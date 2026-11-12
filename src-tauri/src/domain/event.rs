use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::item::{Stage, Status};
use super::turn::Turn;

/// One change in an item's history. Stored as data, not sentences, so the UI can word it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Change {
    Created,
    Stage {
        stage: Stage,
    },
    Status {
        status: Option<Status>,
    },
    Label {
        label: String,
    },
    /// A free-text note from Quarterdeck itself, e.g. "updated the branch". Bookkeeping, so trace;
    /// notes from before records existed were kept as record.
    Note {
        text: String,
    },
    /// A plain sentence about something that mattered, for people to read later.
    Record {
        tag: RecordTag,
        text: String,
    },
    /// The turn passed, and why: what was done, or what ran out.
    Turn {
        turn: Turn,
        why: String,
    },
}

impl Change {
    /// Whether it's bookkeeping (trace) rather than a record written for people.
    pub const fn is_trace(&self) -> bool {
        !matches!(self, Self::Record { .. })
    }
}

/// What a record is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecordTag {
    /// Why the work exists: the problem or need behind it.
    Why,
    /// A decision the user made, e.g. answering a question.
    Decision,
    /// A change of direction.
    Change,
    /// Blocked or unblocked by something outside it.
    Blocked,
    /// The user corrected the picture of it.
    Correction,
    /// What the finished work changed for the user.
    Delivered,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemEvent {
    pub id: i64,
    pub item_id: String,
    pub at: DateTime<Utc>,
    pub change: Change,
    /// Bookkeeping rather than record.
    pub trace: bool,
}
