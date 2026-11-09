//! The `[github]` settings for what Claude fixes on PRs by itself.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct FixSettings {
    /// When CI fails on a PR nobody is on, and no `ci_failed` schedule covers it, hand it to Claude.
    #[serde(rename = "fix_ci")]
    pub ci: bool,
    /// When a PR gets review comments, hand them to Claude: to the session on it, or, if nobody is
    /// on it and no `pr_new_comments` schedule covers them, a new one.
    #[serde(rename = "fix_reviews")]
    pub reviews: bool,
    /// Claude Code tools those runs may use, besides the Quarterdeck tools.
    #[serde(rename = "fix_ci_tools")]
    pub tools: Vec<String>,
}

impl Default for FixSettings {
    fn default() -> Self {
        Self {
            ci: true,
            reviews: true,
            tools: Vec::new(),
        }
    }
}
