use tauri::State;

use crate::app::App;
use crate::domain::outage::Outage;
use crate::error::AppResult;
use crate::scheduler::outage;

/// Checks now whether the agent can run: the outage still going on, or nothing once it's back.
#[tauri::command]
pub async fn check_agent(app: State<'_, App>) -> AppResult<Option<Outage>> {
    outage::check_now(&app.db, &app.settings()?).await
}
