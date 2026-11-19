use chrono::{Duration, Utc};
use tauri::State;

use crate::app::App;
use crate::config::{pause, schedules};
use crate::domain::archive;
use crate::domain::item::WorkItem;
use crate::error::AppResult;
use crate::store::items;
use crate::views::agent::{self, AgentView};
use crate::views::today::{self, TodayView};

#[tauri::command]
pub async fn today(app: State<'_, App>) -> AppResult<TodayView> {
    let settings = app.settings()?;
    app.expire(&settings).await?;
    app.take_inbox(&settings).await?;
    today::build(&app.db, settings.priority_limit, Utc::now()).await
}

/// Work items in priority order, less the archived ones. Board and Priorities share this.
#[tauri::command]
pub async fn work_items(app: State<'_, App>) -> AppResult<Vec<WorkItem>> {
    board_or_archive(&app, false).await
}

/// Done items that have left the board, most recently done first.
#[tauri::command]
pub async fn archived_items(app: State<'_, App>) -> AppResult<Vec<WorkItem>> {
    board_or_archive(&app, true).await
}

async fn board_or_archive(app: &App, archived: bool) -> AppResult<Vec<WorkItem>> {
    let after = Duration::days(app.settings()?.archive_done_days);
    let all = items::list(&app.db).await?;
    Ok(archive::select(all, archived, Utc::now(), after))
}

#[tauri::command]
pub async fn agent(app: State<'_, App>) -> AppResult<AgentView> {
    let settings = app.settings()?;
    app.expire(&settings).await?;
    let loaded = schedules::list(&app.config)?;
    let paused = pause::current(&app.config, Utc::now())?;
    agent::build(&app.db, loaded, paused, &settings.turns).await
}
