//! Fires cron schedules on time on this machine, and records the ones that were missed.

mod agent_prompt;
mod answers;
mod branch;
mod ci;
mod connectors;
mod dispatch;
mod due;
mod events;
mod failed;
mod focus;
mod handoff;
mod keeper;
mod outage;
mod peers;
mod queue;
mod run_turn;
mod sources;
mod thread;
mod tick;
mod undelivered;
mod updates;
mod wake;

use crate::domain::schedule::Schedule;

use std::time::Duration;

use tauri::{AppHandle, Manager as _};

use crate::app::App;
use crate::store::meta;

const LAST_ERROR: &str = "scheduler.last_error";

const EVERY: Duration = Duration::from_secs(30);

/// Runs a schedule now, in the background.
pub use focus::sync_on_focus;

pub fn run_now(handle: &AppHandle, schedule: Schedule) {
    tick::start(handle, schedule, None);
}

/// Starts the scheduler loop for as long as the app runs.
pub fn spawn(handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            // One bad tick (e.g. a half-written settings file) must not stop the loop; the error is
            // kept for diagnostics instead.
            let error = tick::tick(&handle)
                .await
                .err()
                .map(|e| e.to_string())
                .unwrap_or_default();
            drop(meta::set_text(&handle.state::<App>().db, LAST_ERROR, &error).await);
            tokio::time::sleep(EVERY).await;
        }
    });
}
