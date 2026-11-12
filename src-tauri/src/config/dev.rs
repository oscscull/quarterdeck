//! The `[dev]` section of `quarterdeck.toml`, read only by dev builds.

use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DevSettings {
    /// Let a dev build start Claude. Off by default, so a dev build working on a copy of real data
    /// can't resume real sessions or push to real PRs.
    pub run_claude: bool,
    /// Let a dev build run script schedules. Off by default for the same reason.
    pub run_scripts: bool,
}
