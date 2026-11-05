//! Starts the next queued item once the one before it is in review or done.

use tauri::{AppHandle, Manager as _};

use crate::app::App;
use crate::config::prompts::{Prompt, render};
use crate::config::settings::Settings;
use crate::error::AppResult;
use crate::queue::{self, Next};
use crate::store::items;
use crate::turns::Clock;

pub async fn advance(handle: &AppHandle, settings: &Settings) -> AppResult<()> {
    if settings.queue.items.is_empty() {
        return Ok(());
    }
    let app = handle.state::<App>();
    let all = items::list(&app.db).await?;
    let Next::Start(item) = queue::next(&settings.queue.items, &all) else {
        return Ok(());
    };
    let message = render(&app.config, Prompt::Queue, &[])?;
    queue::start(&app.db, item, &message, Clock::now(&settings.turns)).await
}
