//! A schedule that runs a shell script instead of asking Claude.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// What to tell the user about a script's runs. A Claude run always reports.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Report {
    /// Every run goes in "From Claude", and failures are raised as urgent.
    Always,
    /// Only failures, raised as urgent.
    #[default]
    Failures,
    /// Nothing: runs are only in the run history.
    Never,
}

/// How a script schedule runs. The schedule's body is the script.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Script {
    /// The folder it runs in; `~` is the home folder. Unset, the home folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// It's stopped and counts as failed after this long.
    #[serde(default = "default_timeout")]
    pub timeout_minutes: u32,
    #[serde(default)]
    pub report: Report,
}

const fn default_timeout() -> u32 {
    10
}
