//! Runs connector syncs on their own intervals, in the background.

use chrono::{DateTime, Duration, Utc};
use tauri::{AppHandle, Emitter as _, Manager as _};

use crate::app::App;
use crate::config::settings::Settings;
use crate::connectors::{calendar, github, status};
use crate::error::AppResult;
use crate::lasting::{Card, Lasting};
use crate::store::meta;

/// The screen reloads when it hears this, so a sync's changes show without waiting.
pub const SYNCED: &str = "synced";

/// True if `name` hasn't synced within `gap`, claiming the slot so the next tick doesn't start
/// a second sync.
pub(super) async fn claim(handle: &AppHandle, name: &str, gap: Duration) -> AppResult<bool> {
    let app = handle.state::<App>();
    let key = format!("connector.{name}.last_sync");
    let now = Utc::now();
    let last = meta::time(&app.db, &key).await?;
    if last.is_some_and(|t| now - t < gap) {
        return Ok(false);
    }
    meta::set_time(&app.db, &key, now).await?;
    Ok(true)
}

async fn record(handle: &AppHandle, name: &str, error: Option<&str>) {
    let state = handle.state::<App>();
    // Status is for display; failing to save it must not stop the sync loop.
    drop(status::record(&state.db, name, error).await);
}

/// GitHub syncs that keep failing leave PRs and issues stale, so a lasting failure is urgent.
async fn raise_if_lasting(handle: &AppHandle, error: Option<&str>, after: Duration) {
    let db = &handle.state::<App>().db;
    let lasting = Lasting {
        key: "connector.github.failing".to_owned(),
        after,
    };
    let Some(why) = error else {
        drop(lasting.clear(db).await);
        return;
    };
    let detail = |since: DateTime<Utc>| {
        format!(
            "Every GitHub sync since {} has failed, so PRs and issues aren't following GitHub: {why}",
            since.format("%d %b %H:%M")
        )
    };
    let card = Card {
        title: "GitHub isn't syncing",
        detail: &detail,
        area: "",
    };
    drop(lasting.holds(db, &card).await);
}

/// Syncs GitHub in the background and tells the screen. Its events start schedules next tick.
pub(super) fn spawn_github(handle: &AppHandle, failing_after: Duration) {
    let handle = handle.clone();
    tauri::async_runtime::spawn(async move {
        let error = github::sync(&handle).await.err().map(|e| e.to_string());
        record(&handle, github::SOURCE, error.as_deref()).await;
        raise_if_lasting(&handle, error.as_deref(), failing_after).await;
        // The screen also reloads on its own timer, so a missed event only delays it.
        drop(handle.emit(SYNCED, ()));
    });
}

pub async fn sync_if_due(handle: &AppHandle, settings: &Settings) -> AppResult<()> {
    let every = Duration::minutes(settings.github.poll_minutes);
    if settings.github.enabled && claim(handle, github::SOURCE, every).await? {
        spawn_github(handle, Duration::minutes(settings.github.failing_minutes));
    }
    if !settings.calendar.feeds.is_empty()
        && claim(
            handle,
            calendar::SOURCE,
            Duration::minutes(settings.calendar.poll_minutes),
        )
        .await?
    {
        let handle = handle.clone();
        tauri::async_runtime::spawn(async move {
            let result = calendar::sync(&handle).await;
            let error = result.err().map(|e| e.to_string());
            record(&handle, calendar::SOURCE, error.as_deref()).await;
        });
    }
    Ok(())
}
