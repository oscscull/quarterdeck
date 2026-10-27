//! The `[github]` section of `quarterdeck.toml`.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::branches::BranchSettings;
use super::issues::IssueSettings;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GithubSettings {
    /// Uses the `gh` command's existing login.
    pub enabled: bool,
    pub poll_minutes: i64,
    /// Coming back to the window syncs at once, unless a sync ran this recently.
    pub focus_sync_seconds: i64,
    /// Area for PRs from repos not listed in `areas`.
    pub default_area: String,
    /// Owner (`my-org`) or repo (`my-org/app`) to area id. A repo entry beats its owner's.
    pub areas: BTreeMap<String, String>,
    /// Owners or repos whose PRs are skipped entirely.
    pub ignore: Vec<String>,
    /// Owners or repos where the user reviews their own PRs, so review waits on them.
    pub self_review: Vec<String>,
    /// When CI fails on a PR nobody is on, and no `ci_failed` schedule covers it, hand it to Claude.
    pub fix_ci: bool,
    /// Claude Code tools that run may use, besides the Quarterdeck tools.
    pub fix_ci_tools: Vec<String>,
    /// Updating PRs that fall behind, and fixing ones that conflict.
    #[serde(flatten)]
    pub branches: BranchSettings,
    /// Owner or repo to the names of checks that only inform (e.g. a visual-diff report). Their
    /// failures don't count as CI failing; they're noted on the item instead.
    pub informational_checks: BTreeMap<String, Vec<String>>,
    pub issues: IssueSettings,
}

impl Default for GithubSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_minutes: 5,
            focus_sync_seconds: 30,
            default_area: "work".to_owned(),
            areas: BTreeMap::new(),
            ignore: Vec::new(),
            self_review: Vec::new(),
            fix_ci: true,
            fix_ci_tools: Vec::new(),
            branches: BranchSettings::default(),
            informational_checks: BTreeMap::new(),
            issues: IssueSettings::default(),
        }
    }
}

fn owner(repo: &str) -> &str {
    repo.split_once('/').map_or(repo, |(o, _)| o)
}

/// Whether `repo`, or its owner, is in `list`.
pub(super) fn listed(list: &[String], repo: &str) -> bool {
    list.iter().any(|i| i == repo || i == owner(repo))
}

impl GithubSettings {
    pub fn area_for(&self, repo: &str) -> &str {
        self.areas
            .get(repo)
            .or_else(|| self.areas.get(owner(repo)))
            .unwrap_or(&self.default_area)
    }

    /// Informational check names for a repo: its owner's and its own.
    pub fn informational(&self, repo: &str) -> Vec<String> {
        [owner(repo), repo]
            .iter()
            .filter_map(|key| self.informational_checks.get(*key))
            .flatten()
            .cloned()
            .collect()
    }

    pub fn ignores(&self, repo: &str) -> bool {
        listed(&self.ignore, repo)
    }

    pub fn self_reviews(&self, repo: &str) -> bool {
        listed(&self.self_review, repo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_beats_owner_beats_default() {
        let g: GithubSettings = toml::from_str(
            "default_area = \"work\"\nignore = [\"spam\"]\n[areas]\nme = \"projects\"\n\"me/day-job\" = \"work\"",
        )
        .unwrap();
        assert_eq!(g.area_for("me/day-job"), "work");
        assert_eq!(g.area_for("me/toy"), "projects");
        assert_eq!(g.area_for("acme/app"), "work");
        assert!(g.ignores("spam/thing"));
    }
}
