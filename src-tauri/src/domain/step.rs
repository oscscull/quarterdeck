use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// What kind of thing a live run just did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepKind {
    /// Claude Code's own one-line summary of what it's doing.
    Doing,
    /// A tool call; `tool` says which.
    Tool,
    /// Something Claude wrote.
    Said,
    /// A tool call that came back with an error.
    Failed,
    /// A line a script printed; `tool` is `stderr` for its error output.
    Output,
}

/// One step of a live run, short enough to show on one line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub at: DateTime<Utc>,
    pub kind: StepKind,
    pub tool: Option<String>,
    pub text: String,
}
