//! The `[github]` settings for keeping PRs mergeable while they wait.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct BranchSettings {
    /// When a PR falls behind a base it must be up to date with, update its branch (`gh pr
    /// update-branch`), unless a `pr_behind` schedule covers it.
    pub update_behind: bool,
    /// When a PR nobody is on starts conflicting with its base, and no `pr_conflicts` schedule
    /// covers it, hand it to Claude.
    pub fix_conflicts: bool,
}

impl Default for BranchSettings {
    fn default() -> Self {
        Self {
            update_behind: true,
            fix_conflicts: true,
        }
    }
}
