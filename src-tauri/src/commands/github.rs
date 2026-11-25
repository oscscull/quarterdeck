use tauri::AppHandle;

use crate::error::AppResult;
use crate::scheduler::{connectors, syncing};

/// Starts a GitHub sync now. False if GitHub is turned off.
#[tauri::command]
pub async fn sync_github(handle: AppHandle) -> AppResult<bool> {
    connectors::sync_github_now(&handle).await
}

/// Whether a GitHub sync is running, for a screen that opens mid-sync.
#[tauri::command]
pub fn github_syncing() -> bool {
    syncing::running()
}
