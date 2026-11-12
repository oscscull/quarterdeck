use tauri::State;

use crate::app::App;
use crate::briefing::correct::{self, Correction};
use crate::briefing::view::{self, BriefingView};
use crate::error::AppResult;

/// The Briefing tab's data on this machine.
#[tauri::command]
pub async fn briefing(app: State<'_, App>) -> AppResult<BriefingView> {
    let settings = app.settings()?;
    view::build(&app.db, &settings, None).await
}

/// Answers one message from the Briefing's correction box.
#[tauri::command]
pub async fn correct_briefing(app: State<'_, App>, correction: Correction) -> AppResult<String> {
    let settings = app.settings()?;
    correct::reply(&app.db, &app.config, &settings, &correction).await
}
