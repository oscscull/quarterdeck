use chrono::{Duration, Utc};

use crate::config::Config;
use crate::config::inbox;
use crate::config::settings::{self, Settings};
use crate::error::AppResult;
use crate::store::Db;
use crate::store::expire::{Ttl, expire};
use crate::store::urgent;

/// Shared state behind every command.
#[derive(Debug)]
pub struct App {
    pub db: Db,
    pub config: Config,
}

impl App {
    /// Current settings, re-read so file edits apply immediately.
    pub fn settings(&self) -> AppResult<Settings> {
        settings::load(&self.config)
    }

    /// Turns any files agents dropped in `inbox/` into urgent items.
    /// Returns how many items were raised.
    pub async fn take_inbox(&self, settings: &Settings) -> AppResult<usize> {
        let raised = inbox::take(&self.config)?;
        let count = raised.len();
        for r in raised {
            let area = settings.known_area(&r.area);
            urgent::raise(&self.db, &r.into_urgent(area, Utc::now())).await?;
        }
        Ok(count)
    }

    /// Deletes stored output that has outlived its TTL.
    pub async fn expire(&self, settings: &Settings) -> AppResult<()> {
        let ttl = Ttl {
            activity: Duration::days(settings.activity_ttl_days),
            runs: Duration::days(settings.run_ttl_days),
            events: Duration::days(settings.event_ttl_days),
            done_todos: Duration::days(settings.done_todo_ttl_days),
            done_items: Duration::days(settings.done_item_ttl_days),
        };
        expire(&self.db, Utc::now(), ttl).await
    }
}
