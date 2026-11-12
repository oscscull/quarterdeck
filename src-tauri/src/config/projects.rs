//! `[[projects]]` in `quarterdeck.toml`: where a GitHub repo is checked out on this machine. Work Quarterdeck
//! starts on that repo's PRs runs there, in a worktree of its own.

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Project {
    /// `owner/repo` on GitHub.
    pub repo: String,
    /// The checkout's folder. `~` means the home folder.
    pub path: String,
    /// Folders inside a worktree, relative to its root, deleted after every run there, e.g. build
    /// output such as `target` or `dist`. The next run rebuilds what it needs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clean_after_run: Vec<String>,
}

impl Project {
    /// The checkout folder with `~` expanded.
    pub fn dir(&self) -> PathBuf {
        match (self.path.strip_prefix("~/"), dirs::home_dir()) {
            (Some(rest), Some(home)) => home.join(rest),
            _ => PathBuf::from(&self.path),
        }
    }
}

/// The project for a repo, ignoring case as GitHub does.
pub fn find<'a>(projects: &'a [Project], repo: &str) -> Option<&'a Project> {
    projects.iter().find(|p| p.repo.eq_ignore_ascii_case(repo))
}
