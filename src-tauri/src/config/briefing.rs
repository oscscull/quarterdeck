//! The `[briefing]` settings: when the Briefing is rewritten and how long its parts last.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct BriefingSettings {
    /// How long an area's work must hold still after a change before its briefing is rewritten, so a
    /// burst of changes costs one run.
    pub debounce_minutes: i64,
    /// How long finished work stays in "Just done".
    pub just_done_days: i64,
    /// How long a reference nothing open uses is kept.
    pub reference_ttl_days: i64,
}

impl Default for BriefingSettings {
    fn default() -> Self {
        Self {
            debounce_minutes: 10,
            just_done_days: 2,
            reference_ttl_days: 14,
        }
    }
}
