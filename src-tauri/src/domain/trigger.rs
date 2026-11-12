//! When a schedule runs: on a clock, or when an event happens.
#![expect(
    unused_qualifications,
    reason = "schemars' derive qualifies the type inside `Trigger::Event`"
)]

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Starts a schedule when an event happens in its area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OnEvent {
    /// The event's name, e.g. `github.ci_failed` or `item.turn` (see `list_event_kinds`).
    #[serde(alias = "kind")]
    pub on: String,
    /// Fields the event must have, e.g. `{ to = "agent" }`. Values may use `*` as a wildcard.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub when: BTreeMap<String, String>,
    /// Minutes after a run on the same subject (item, schedule…) before another may start.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub cooldown: u32,
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes a reference"
)]
const fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// When a schedule fires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Trigger {
    Cron { expr: String },
    Event(OnEvent),
}

impl Trigger {
    /// Old files named GitHub events without their source, e.g. `ci_failed`.
    #[must_use]
    pub fn normalised(self) -> Self {
        match self {
            Self::Event(mut e) => {
                if !e.on.contains('.') {
                    e.on = format!("github.{}", e.on);
                }
                Self::Event(e)
            }
            Self::Cron { expr } => Self::Cron { expr },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_event_names_gain_their_source() {
        let old: Trigger = toml::from_str("type = \"event\"\nkind = \"ci_failed\"").unwrap();
        let Trigger::Event(e) = old.normalised() else {
            panic!("not an event")
        };
        assert_eq!(e.on, "github.ci_failed");
        assert!(e.when.is_empty());
    }
}
