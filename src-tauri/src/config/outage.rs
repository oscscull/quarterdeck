//! The `[outage]` section of `quarterdeck.toml`: what happens while the agent can't run at all.

use std::time::Duration;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct OutageSettings {
    /// The wait before the first check after the agent couldn't run. Each later wait doubles.
    pub first_retry_seconds: u64,
    /// The longest wait between checks.
    pub max_retry_seconds: u64,
    /// `host:port` the agent needs; an empty value checks only that `claude` is there.
    pub probe: String,
    /// `host:port` for GitHub, which worktrees and pushes need; empty skips it.
    pub github_probe: String,
    /// How long to wait for `probe` to answer.
    pub probe_timeout_seconds: u64,
}

impl OutageSettings {
    /// The wait after `gap`: doubled, between the first and the longest.
    pub fn next_gap(&self, gap: Option<u64>) -> u64 {
        let first = self.first_retry_seconds.max(1);
        gap.map_or(first, |g| g.saturating_mul(2))
            .clamp(first, self.max_retry_seconds.max(first))
    }

    pub const fn timeout(&self) -> Duration {
        Duration::from_secs(self.probe_timeout_seconds)
    }
}

impl Default for OutageSettings {
    fn default() -> Self {
        Self {
            first_retry_seconds: 60,
            max_retry_seconds: 900,
            probe: "api.anthropic.com:443".to_owned(),
            github_probe: "github.com:443".to_owned(),
            probe_timeout_seconds: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_double_up_to_the_longest() {
        let s = OutageSettings::default();
        assert_eq!(s.next_gap(None), 60);
        assert_eq!(s.next_gap(Some(60)), 120);
        assert_eq!(s.next_gap(Some(600)), 900);
        assert_eq!(s.next_gap(Some(900)), 900);
    }
}
