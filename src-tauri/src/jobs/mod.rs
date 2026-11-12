//! Running a schedule: through Claude, or as a shell script. Both start and finish a run the same
//! way, so history, events and reporting don't depend on which it is.

pub mod approval;
mod script;
mod settle;
mod shell;

pub use settle::{Begun, Ended, begin, settle, summary_of};

use tauri::AppHandle;

use crate::domain::fired::Fired;
use crate::domain::schedule::Schedule;
use crate::error::AppResult;

/// Runs `schedule` now; `cause` is the event that started it, if one did.
pub async fn run(handle: &AppHandle, schedule: &Schedule, cause: Option<&Fired>) -> AppResult<()> {
    if schedule.script.is_some() {
        script::run(handle, schedule, cause).await
    } else {
        crate::claude::runner::run(handle, schedule, cause).await
    }
}
