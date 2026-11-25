//! Whether a GitHub sync is running, so only one runs at a time and the screen can show it.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter as _};

/// The screen shows a sync as running from this event until `SYNCED`.
pub const SYNCING: &str = "syncing";

static RUNNING: AtomicBool = AtomicBool::new(false);

pub fn running() -> bool {
    RUNNING.load(Ordering::Acquire)
}

/// Claims the one running slot and tells the screen. False if a sync is already running.
pub(super) fn start(handle: &AppHandle) -> bool {
    if RUNNING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return false;
    }
    // A missed event only means the screen shows the sync late.
    drop(handle.emit(SYNCING, ()));
    true
}

pub(super) fn finish() {
    RUNNING.store(false, Ordering::Release);
}
