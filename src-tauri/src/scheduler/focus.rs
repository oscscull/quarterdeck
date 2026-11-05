//! Coming back to the window syncs GitHub at once, so a PR merged in the browser leaves the screen
//! right away instead of at the next poll, and checks again at once whether a down agent is back.

use chrono::Duration;
use tauri::{AppHandle, Manager as _};

use super::connectors::{claim, spawn_github};
use crate::app::App;
use crate::connectors::github;
use crate::error::AppResult;

/// Called from the window's focus event, so the check runs in the background.
pub fn sync_on_focus(handle: &AppHandle) {
    let handle = handle.clone();
    tauri::async_runtime::spawn(async move {
        // The regular poll and checks still run if these fail.
        drop(sync_if_stale(&handle).await);
        drop(super::outage::recheck(&handle).await);
    });
}

async fn sync_if_stale(handle: &AppHandle) -> AppResult<()> {
    let settings = handle.state::<App>().settings()?.github;
    let gap = Duration::seconds(settings.focus_sync_seconds);
    if settings.enabled && claim(handle, github::SOURCE, gap).await? {
        spawn_github(handle, Duration::minutes(settings.failing_minutes));
    }
    Ok(())
}
