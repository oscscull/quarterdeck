//! Brings a PR's branch up to date with the branch it merges into.

use super::fetch::gh;
use crate::error::AppResult;

/// Merges the base branch into the PR's branch on GitHub, as its "Update branch" button does.
pub async fn update_branch(url: &str) -> AppResult<()> {
    gh(&["pr", "update-branch", url]).await.map(drop)
}
