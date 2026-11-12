use chrono::Utc;
use tauri::State;

use crate::app::App;
use crate::config::{pause, schedules};
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

/// All work items in priority order. Board and Priorities share this.
#[tauri::command]
pub async fn work_items(app: State<'_, App>) -> AppResult<Vec<WorkItem>> {
    items::list(&app.db).await
}

#[tauri::command]
pub async fn agent(app: State<'_, App>) -> AppResult<AgentView> {
    let settings = app.settings()?;
    app.expire(&settings).await?;
    let loaded = schedules::list(&app.config)?;
    let paused = pause::current(&app.config, Utc::now())?;
    agent::build(&app.db, loaded, paused, &settings.turns).await
}
