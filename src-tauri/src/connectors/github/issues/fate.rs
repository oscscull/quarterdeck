//! Whether GitHub says an issue is still open. An item is only closed on GitHub's word.

use super::super::fetch::gh;
use crate::error::{AppError, AppResult};

/// True when the issue with external id `owner/repo#12` is closed.
pub async fn closed(external_id: &str) -> AppResult<bool> {
    let (repo, number) = external_id
        .split_once('#')
        .ok_or_else(|| AppError::Connector(format!("not an issue id: {external_id}")))?;
    let args = [
        "issue", "view", number, "--repo", repo, "--json", "state", "--jq", ".state",
    ];
    match gh(&args).await?.as_str() {
        "CLOSED" => Ok(true),
        "OPEN" => Ok(false),
        other => Err(AppError::Connector(format!(
            "gh: unknown issue state {other:?}"
        ))),
    }
}
