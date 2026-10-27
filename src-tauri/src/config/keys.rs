//! The plain settings in `quarterdeck.toml` an agent may change by name. `claude_command` is left
//! out on purpose: whoever sets it chooses what program Quarterdeck runs.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use toml_edit::{Array, Item, value};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
pub enum SettingKey {
    /// How many priorities Today shows (number).
    #[serde(rename = "priority_limit")]
    PriorityLimit,
    /// How many earlier "Ask Claude" messages go with each new one (number).
    #[serde(rename = "chat_history_messages")]
    ChatHistoryMessages,
    /// Hours a PR waits on reviewers before Today suggests a nudge (number).
    #[serde(rename = "nudge_after_hours")]
    NudgeAfterHours,
    /// Days Claude's updates ("From Claude") are kept (number).
    #[serde(rename = "activity_ttl_days")]
    ActivityTtlDays,
    /// Days finished runs are kept (number).
    #[serde(rename = "run_ttl_days")]
    RunTtlDays,
    /// Days events (what fired, what it started) are kept (number).
    #[serde(rename = "event_ttl_days")]
    EventTtlDays,
    /// Days done to-dos are kept (number).
    #[serde(rename = "done_todo_ttl_days")]
    DoneTodoTtlDays,
    /// Days done items and their history are kept (number).
    #[serde(rename = "done_item_ttl_days")]
    DoneItemTtlDays,
    /// Start Quarterdeck at login (true/false).
    #[serde(rename = "launch_at_login")]
    LaunchAtLogin,
    /// Claude Code permission mode for work in a project: "auto" or "acceptEdits".
    #[serde(rename = "project_permission_mode")]
    ProjectPermissionMode,
    /// Sync GitHub at all (true/false).
    #[serde(rename = "github.enabled")]
    GithubEnabled,
    /// Minutes between GitHub syncs (number).
    #[serde(rename = "github.poll_minutes")]
    GithubPollMinutes,
    /// Coming back to the window syncs GitHub, at most this often in seconds (number).
    #[serde(rename = "github.focus_sync_seconds")]
    GithubFocusSyncSeconds,
    /// Area id for PRs and issues from owners or repos with no area mapped (text).
    #[serde(rename = "github.default_area")]
    GithubDefaultArea,
    /// Hand CI failures nobody is on to Claude (true/false).
    #[serde(rename = "github.fix_ci")]
    GithubFixCi,
    /// Claude Code tools a CI fix may use, e.g. "Bash(gh pr checks:*)" (list of text).
    #[serde(rename = "github.fix_ci_tools")]
    GithubFixCiTools,
    /// Update a PR's branch when it falls behind a base it must be up to date with (true/false).
    #[serde(rename = "github.update_behind")]
    GithubUpdateBehind,
    /// Hand PRs that start conflicting with their base to Claude (true/false).
    #[serde(rename = "github.fix_conflicts")]
    GithubFixConflicts,
    /// Issues with any of these labels never become items (list of text).
    #[serde(rename = "github.issues.skip_labels")]
    GithubIssueSkipLabels,
    /// Minutes between calendar syncs (number).
    #[serde(rename = "calendar.poll_minutes")]
    CalendarPollMinutes,
    /// How many days of meetings to bring in (number).
    #[serde(rename = "calendar.days_ahead")]
    CalendarDaysAhead,
    /// The on-top countdown before meetings (true/false).
    #[serde(rename = "alerts.enabled")]
    AlertsEnabled,
    /// Minutes before a meeting the countdown appears (number).
    #[serde(rename = "alerts.lead_minutes")]
    AlertsLeadMinutes,
    /// Minutes the countdown stays after a meeting starts (number).
    #[serde(rename = "alerts.linger_minutes")]
    AlertsLingerMinutes,
    /// Check for new Quarterdeck versions (true/false).
    #[serde(rename = "updates.enabled")]
    UpdatesEnabled,
    /// Minutes between update checks (number).
    #[serde(rename = "updates.check_minutes")]
    UpdatesCheckMinutes,
    /// Let a dev build run Claude (true/false).
    #[serde(rename = "dev.run_claude")]
    DevRunClaude,
    /// Let a dev build run script schedules (true/false).
    #[serde(rename = "dev.run_scripts")]
    DevRunScripts,
}

impl SettingKey {
    /// Where the setting lives in `quarterdeck.toml`.
    pub fn path(self) -> Vec<&'static str> {
        let name = match self {
            Self::PriorityLimit => "priority_limit",
            Self::ChatHistoryMessages => "chat_history_messages",
            Self::NudgeAfterHours => "nudge_after_hours",
            Self::ActivityTtlDays => "activity_ttl_days",
            Self::RunTtlDays => "run_ttl_days",
            Self::EventTtlDays => "event_ttl_days",
            Self::DoneTodoTtlDays => "done_todo_ttl_days",
            Self::DoneItemTtlDays => "done_item_ttl_days",
            Self::LaunchAtLogin => "launch_at_login",
            Self::ProjectPermissionMode => "project_permission_mode",
            Self::GithubEnabled => "github.enabled",
            Self::GithubPollMinutes => "github.poll_minutes",
            Self::GithubFocusSyncSeconds => "github.focus_sync_seconds",
            Self::GithubDefaultArea => "github.default_area",
            Self::GithubFixCi => "github.fix_ci",
            Self::GithubFixCiTools => "github.fix_ci_tools",
            Self::GithubUpdateBehind => "github.update_behind",
            Self::GithubFixConflicts => "github.fix_conflicts",
            Self::GithubIssueSkipLabels => "github.issues.skip_labels",
            Self::CalendarPollMinutes => "calendar.poll_minutes",
            Self::CalendarDaysAhead => "calendar.days_ahead",
            Self::AlertsEnabled => "alerts.enabled",
            Self::AlertsLeadMinutes => "alerts.lead_minutes",
            Self::AlertsLingerMinutes => "alerts.linger_minutes",
            Self::UpdatesEnabled => "updates.enabled",
            Self::UpdatesCheckMinutes => "updates.check_minutes",
            Self::DevRunClaude => "dev.run_claude",
            Self::DevRunScripts => "dev.run_scripts",
        };
        name.split('.').collect()
    }
}

/// A JSON value as a TOML one. Whether it suits the setting is checked when the file reloads.
pub fn toml_value(v: &Value) -> AppResult<Item> {
    match v {
        Value::Bool(b) => Ok(value(*b)),
        Value::Number(n) => n
            .as_i64()
            .map(value)
            .ok_or_else(|| AppError::BadInput(format!("{n} isn't a whole number"))),
        Value::String(s) => Ok(value(s.as_str())),
        Value::Array(list) => {
            let texts: Option<Array> = list.iter().map(Value::as_str).collect();
            texts
                .map(value)
                .ok_or_else(|| AppError::BadInput("lists hold text only".to_owned()))
        }
        Value::Null | Value::Object(_) => {
            Err(AppError::BadInput(format!("{v} isn't a setting value")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_name_their_place_in_the_file() {
        let key: SettingKey = serde_json::from_str("\"github.issues.skip_labels\"").unwrap();
        assert_eq!(key.path(), ["github", "issues", "skip_labels"]);
        assert!(serde_json::from_str::<SettingKey>("\"claude_command\"").is_err());
        assert!(toml_value(&serde_json::json!(["a", 1])).is_err());
    }
}
