use super::projects::Project;
use super::queue::QueueSettings;
use serde::{Deserialize, Serialize};

use super::Config;
use super::alerts::AlertSettings;
use super::calendar::CalendarSettings;
use super::dev::DevSettings;
use super::github::GithubSettings;
use super::outage::OutageSettings;
use super::sync::SyncSettings;
use super::turns::TurnSettings;
use super::updates::UpdateSettings;
use crate::error::{AppError, AppResult};
use crate::profile;

/// A part of life Quarterdeck keeps separate, e.g. Work, Personal projects, Life.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Area {
    pub id: String,
    pub name: String,
    /// Dot colour in the UI, as a CSS hex value.
    pub color: String,
    /// The machine that runs this area's schedules.
    pub node: String,
}

/// Mechanical settings from `quarterdeck.toml`. Anything missing falls back to a default, so a
/// hand-edited file with a line removed still loads.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub node: String,
    pub priority_limit: usize,
    /// How many earlier chat messages go to Claude with each new one.
    pub chat_history_messages: usize,
    pub claude_command: String,
    pub activity_ttl_days: i64,
    pub run_ttl_days: i64,
    pub event_ttl_days: i64,
    pub done_todo_ttl_days: i64,
    pub done_item_ttl_days: i64,
    pub areas: Vec<Area>,
    pub github: GithubSettings,
    pub calendar: CalendarSettings,
    pub alerts: AlertSettings,
    /// Start Quarterdeck when you log in, so schedules and alerts keep working.
    pub launch_at_login: bool,
    pub sync: SyncSettings,
    pub turns: TurnSettings,
    pub outage: OutageSettings,
    pub projects: Vec<Project>,
    /// Claude Code's `--permission-mode` for work in a project, e.g. `auto` (Claude decides what's
    /// safe, as in the user's own sessions) or `acceptEdits` (only the project's allow rules).
    pub project_permission_mode: String,
    pub queue: QueueSettings,
    pub updates: UpdateSettings,
    pub dev: DevSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            node: "this-mac".to_owned(),
            priority_limit: 3,
            chat_history_messages: 20,
            claude_command: "claude".to_owned(),
            activity_ttl_days: 7,
            run_ttl_days: 14,
            event_ttl_days: 3,
            done_todo_ttl_days: 1,
            done_item_ttl_days: 30,
            areas: Vec::new(),
            github: GithubSettings::default(),
            calendar: CalendarSettings::default(),
            alerts: AlertSettings::default(),
            launch_at_login: true,
            sync: SyncSettings::default(),
            turns: TurnSettings::default(),
            outage: OutageSettings::default(),
            projects: Vec::new(),
            project_permission_mode: "auto".to_owned(),
            queue: QueueSettings::default(),
            updates: UpdateSettings::default(),
            dev: DevSettings::default(),
        }
    }
}

impl Settings {
    /// The machine an area runs on; this machine for an unknown area.
    pub fn node_for(&self, area: &str) -> &str {
        self.areas
            .iter()
            .find(|a| a.id == area)
            .map_or(self.node.as_str(), |a| a.node.as_str())
    }

    /// The command that runs Claude. A dev build runs it only when `[dev] run_claude` is on.
    pub fn claude(&self) -> AppResult<&str> {
        if profile::DEV && !self.dev.run_claude {
            return Err(AppError::Claude(
                "this dev build doesn't run Claude; set [dev] run_claude = true in its quarterdeck.toml"
                    .to_owned(),
            ));
        }
        Ok(&self.claude_command)
    }

    /// Whether script schedules may run. A dev build runs them only when `[dev] run_scripts` is on.
    pub fn scripts(&self) -> AppResult<()> {
        if profile::DEV && !self.dev.run_scripts {
            return Err(AppError::BadInput(
                "this dev build doesn't run scripts; set [dev] run_scripts = true in its quarterdeck.toml"
                    .to_owned(),
            ));
        }
        Ok(())
    }

    /// `area` if it is configured (or none are), otherwise the first area.
    pub fn known_area(&self, area: &str) -> String {
        match self.areas.first() {
            Some(first) if !self.areas.iter().any(|a| a.id == area) => first.id.clone(),
            Some(_) | None => area.to_owned(),
        }
    }
}

pub fn load(config: &Config) -> AppResult<Settings> {
    let text = std::fs::read_to_string(config.root().join("quarterdeck.toml"))?;
    Ok(toml::from_str(&text)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        let area = |id: &str, node: &str| Area {
            id: id.to_owned(),
            name: id.to_owned(),
            color: String::new(),
            node: node.to_owned(),
        };
        Settings {
            node: "this-mac".to_owned(),
            priority_limit: 3,
            chat_history_messages: 20,
            claude_command: String::new(),
            activity_ttl_days: 7,
            run_ttl_days: 14,
            event_ttl_days: 3,
            done_todo_ttl_days: 1,
            done_item_ttl_days: 30,
            areas: vec![area("work", "work-mac"), area("life", "home-mac")],
            github: GithubSettings::default(),
            calendar: CalendarSettings::default(),
            alerts: AlertSettings::default(),
            launch_at_login: true,
            sync: SyncSettings::default(),
            turns: TurnSettings::default(),
            outage: OutageSettings::default(),
            projects: Vec::new(),
            project_permission_mode: "auto".to_owned(),
            queue: QueueSettings::default(),
            updates: UpdateSettings::default(),
            dev: DevSettings::default(),
        }
    }

    #[test]
    fn areas_pick_their_machine() {
        assert_eq!(settings().node_for("life"), "home-mac");
        assert_eq!(settings().node_for("nope"), "this-mac");
    }

    #[test]
    fn unknown_areas_fall_back_to_the_first() {
        assert_eq!(settings().known_area("life"), "life");
        assert_eq!(settings().known_area("nope"), "work");
        assert_eq!(Settings::default().known_area("life"), "life");
    }

    #[test]
    fn missing_lines_fall_back() {
        let parsed: Settings = toml::from_str("node = \"x\"").unwrap();
        assert_eq!(parsed.priority_limit, 3);
        assert_eq!(parsed.node, "x");
    }

    #[test]
    fn shipped_settings_parse() {
        let parsed: Settings =
            toml::from_str(include_str!("../../defaults/config/quarterdeck.toml")).unwrap();
        assert_eq!(parsed.areas.len(), 3);
    }
}
