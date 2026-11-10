use chrono::Utc;
use tauri::State;

use crate::app::App;
use crate::claude;
use crate::config::prompts::{Prompt, render};
use crate::domain::activity::Activity;
use crate::error::AppResult;
use crate::retry;
use crate::store::{activity, urgent};

#[tauri::command]
pub async fn dismiss_urgent(app: State<'_, App>, id: i64) -> AppResult<()> {
    urgent::dismiss(&app.db, id).await
}

#[tauri::command]
pub async fn dismiss_all_urgent(app: State<'_, App>) -> AppResult<()> {
    urgent::dismiss_all(&app.db).await
}

/// Asks the card's failed run again on the next tick.
#[tauri::command]
pub async fn retry_urgent(app: State<'_, App>, id: i64) -> AppResult<()> {
    retry::request(&app.db, id, &app.settings()?.turns).await
}

/// Hands an urgent item to Claude with the user's instructions. Claude's one-line reply becomes an
/// activity note (which expires like any other) and the item is cleared.
#[tauri::command]
pub async fn resolve_urgent(app: State<'_, App>, id: i64, message: String) -> AppResult<String> {
    let settings = app.settings()?;
    let item = urgent::find(&app.db, id).await?;
    let vars = [
        ("title", item.title.as_str()),
        ("detail", item.detail.as_str()),
        ("message", message.as_str()),
    ];
    let prompt = render(&app.config, Prompt::ResolveUrgent, &vars)?;
    let reply = claude::ask(&settings.claude()?, &prompt).await?;
    let note = Activity {
        id: 0,
        text: reply.clone(),
        area: item.area,
        at: Utc::now(),
        unread: true,
    };
    activity::insert(&app.db, &note).await?;
    urgent::dismiss(&app.db, id).await?;
    Ok(reply)
}
