use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::script::Script;
use super::trigger::{OnEvent, Trigger};

/// Things the GitHub connector detects on a PR by itself, with no tokens spent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    CiFailed,
    CiPassed,
    PrNewComments,
    PrBehind,
    PrConflicts,
}

impl EventKind {
    /// The event's name in the catalogue, e.g. `github.ci_failed`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::CiFailed => "github.ci_failed",
            Self::CiPassed => "github.ci_passed",
            Self::PrNewComments => "github.pr_new_comments",
            Self::PrBehind => "github.pr_behind",
            Self::PrConflicts => "github.pr_conflicts",
        }
    }
}

/// A recurring job: instructions for Claude, or a shell script.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schedule {
    pub id: String,
    pub name: String,
    /// Instructions for Claude, or the script when `script` is set.
    pub prompt: String,
    /// Set for a script schedule: it runs `prompt` in the user's shell, with no Claude involved.
    #[serde(default)]
    pub script: Option<Script>,
    /// Any of these starts it.
    pub triggers: Vec<Trigger>,
    /// Which area this belongs to; the area decides the machine it runs on.
    pub area: String,
    pub enabled: bool,
    /// The plain-English request this schedule was made from, if any.
    pub request: String,
    /// Claude Code tools the run may use without asking, e.g. `Bash(gh pr comment:*)`.
    /// Empty means it can only read and report back.
    #[serde(default)]
    pub allowed_tools: Vec<String>,
}

impl Schedule {
    /// The cron expressions among its triggers.
    pub fn crons(&self) -> impl Iterator<Item = &str> {
        self.triggers.iter().filter_map(|t| match t {
            Trigger::Cron { expr } => Some(expr.as_str()),
            Trigger::Event(_) => None,
        })
    }

    /// The event triggers among its triggers.
    pub fn on_events(&self) -> impl Iterator<Item = &OnEvent> {
        self.triggers.iter().filter_map(|t| match t {
            Trigger::Event(e) => Some(e),
            Trigger::Cron { .. } => None,
        })
    }
}
