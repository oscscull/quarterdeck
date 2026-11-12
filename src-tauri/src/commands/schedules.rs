use chrono::{DateTime, Local, Utc};
use tauri::{AppHandle, Manager as _, State};

use crate::app::App;
use crate::config::{pause, schedules};
use crate::cron::explain::{Explanation, explain};
use crate::domain::fired::{KINDS, Kind};
use crate::domain::schedule::Schedule;
use crate::error::{AppError, AppResult};
use crate::jobs::approval;
use crate::peers;
use crate::scheduler;
use crate::store::runs;

const PREVIEW_RUNS: usize = 3;

#[tauri::command]
pub fn set_schedule_enabled(app: State<'_, App>, id: String, enabled: bool) -> AppResult<()> {
    let mut schedule = schedules::find(&app.config, &id)?;
    schedule.enabled = enabled;
    schedules::save(&app.config, &schedule)
}

/// Deletes a schedule file, its run history and any approval of its script.
#[tauri::command]
pub async fn delete_schedule(app: State<'_, App>, id: String) -> AppResult<()> {
    schedules::delete(&app.config, &id)?;
    approval::forget(&app.db, &id).await?;
    runs::delete_for(&app.db, &id).await
}

/// Approves a script schedule as its file stands now. Only the app's window and peers offer this.
#[tauri::command]
pub async fn approve_script(app: State<'_, App>, id: String) -> AppResult<()> {
    approval::approve(&app.db, &schedules::find(&app.config, &id)?).await
}

/// Writes a schedule file. An empty id creates a new one. The user wrote it here, so a script is
/// approved as saved.
#[tauri::command]
pub async fn save_schedule(app: State<'_, App>, mut schedule: Schedule) -> AppResult<String> {
    schedules::check(&schedule)?;
    if schedule.id.is_empty() {
        if let Some(id) =
            peers::handoff::create_elsewhere_async(app.settings()?, schedule.clone()).await?
        {
            return Ok(id);
        }
        schedule.id = schedules::free_id(&app.config, &schedules::slug(&schedule.name));
    }
    schedules::save(&app.config, &schedule)?;
    approval::approve(&app.db, &schedule).await?;
    Ok(schedule.id)
}

/// Events a trigger can wait for, for the trigger editor.
#[tauri::command]
pub fn event_kinds() -> Vec<Kind> {
    KINDS.to_vec()
}

/// Live explanation while the user edits a cron expression.
#[tauri::command]
pub fn explain_cron(expr: String) -> AppResult<Explanation> {
    explain(&expr, Local::now(), PREVIEW_RUNS)
}

/// Starts a schedule in the background; the Agent view shows it as live until it finishes.
/// A schedule held here but owned by another machine runs there instead.
#[tauri::command]
pub async fn run_schedule_now(handle: AppHandle, id: String) -> AppResult<()> {
    let app = handle.state::<App>();
    let schedule = schedules::find(&app.config, &id)?;
    let settings = app.settings()?;
    if !peers::handoff::run_elsewhere(&app.db, &app.config, &settings, schedule.clone()).await? {
        scheduler::run_now(&handle, schedule);
    }
    Ok(())
}

/// Pauses every schedule until `until` (an RFC 3339 time), or until resumed if `None`.
#[tauri::command]
pub fn pause_all(app: State<'_, App>, until: Option<String>) -> AppResult<()> {
    let until = until
        .map(|text| DateTime::parse_from_rfc3339(&text).map(|t| t.with_timezone(&Utc)))
        .transpose()
        .map_err(|e| AppError::NotFound(format!("a valid pause end time ({e})")))?;
    pause::pause(&app.config, until)
}

#[tauri::command]
pub fn resume_all(app: State<'_, App>) -> AppResult<()> {
    pause::resume(&app.config)
}
