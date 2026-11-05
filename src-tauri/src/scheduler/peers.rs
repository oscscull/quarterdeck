//! Finds every peer (or raises the ones that can't be found), keeps machines agreeing on areas,
//! then moves schedules to the machine that owns them. Detached from the tick, since an offline
//! peer can take seconds to time out.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager as _};

use crate::app::App;
use crate::error::AppResult;
use crate::peers::{areas, handoff, health};

/// Set while a pass runs, so a slow one isn't overlapped by the next tick's.
static BUSY: AtomicBool = AtomicBool::new(false);

pub fn sync(handle: &AppHandle) {
    if BUSY.swap(true, Ordering::AcqRel) {
        return;
    }
    let handle = handle.clone();
    tauri::async_runtime::spawn(async move {
        drop(pass(&handle).await);
        BUSY.store(false, Ordering::Release);
    });
}

async fn pass(handle: &AppHandle) -> AppResult<()> {
    let app = handle.state::<App>();
    let settings = app.settings()?;
    if settings.sync.peers.is_empty() {
        return Ok(());
    }
    health::check(&app.db, &app.config, &settings).await?;
    // A peer found at a new address is saved there, so read the settings again.
    let found = app.settings()?;
    areas::push(&app.db, &found).await?;
    handoff::move_misplaced(&app.db, &app.config, &found).await
}
