//! Changes left in a worktree that no commit holds. Quarterdeck only publishes commits, so anything
//! here would never reach a PR.

use super::Workspace;
use super::git::run;
use crate::error::AppResult;

/// Quarterdeck copies this into every worktree (see `git::prepare`), so it isn't the agent's work.
const OURS: &str = ":(exclude).claude/settings.local.json";

/// The paths with uncommitted changes, tracked or new; empty when the worktree is clean.
pub async fn uncommitted(ws: &Workspace) -> AppResult<Vec<String>> {
    let status = ["status", "--porcelain", "--", ".", OURS];
    let out = run("git", &ws.dir, &status).await?;
    Ok(paths(&out))
}

/// `git status --porcelain` lines (` M src/a.rs`, `?? new.rs`) → their paths. Split off the status
/// code rather than cutting at a column, since `run` trims the first line's leading space.
fn paths(porcelain: &str) -> Vec<String> {
    porcelain
        .lines()
        .filter_map(|line| line.trim_start().split_once(' '))
        .map(|(_, path)| path.trim())
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_porcelain_paths() {
        assert_eq!(
            paths("M src/a.rs\n M b.rs\n?? new.rs\nR  old.rs -> moved.rs\n"),
            ["src/a.rs", "b.rs", "new.rs", "old.rs -> moved.rs"]
        );
        assert_eq!(paths(""), Vec::<String>::new());
    }
}
