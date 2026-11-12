use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// What kind of source a reference is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RefKind {
    /// Something the user said, e.g. a correction on the Briefing.
    YouSaid,
    Ticket,
    Pr,
    /// A comment on a PR, issue or ticket.
    Comment,
    Slack,
    Doc,
    Commit,
    Config,
    Other,
}

/// Who added a reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RefBy {
    You,
    Agent,
}

/// A source the briefing cites, with the data point that matters in it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    pub id: i64,
    pub area: String,
    pub kind: RefKind,
    pub title: String,
    pub url: Option<String>,
    /// The data point, quoted or in a line.
    pub quote: String,
    pub by: RefBy,
    pub at: DateTime<Utc>,
    pub project: Option<String>,
    /// The items it's about.
    pub items: Vec<String>,
}
