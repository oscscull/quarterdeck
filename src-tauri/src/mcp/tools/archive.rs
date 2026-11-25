use chrono::{Duration, Utc};
use schemars::JsonSchema;
use serde::Deserialize;

use super::super::ctx::Ctx;
use super::super::tool::{Tool, async_tool};
use super::in_area;
use crate::domain::archive;
use crate::domain::item::WorkItem;
use crate::error::AppResult;
use crate::store::items;

const LIST_ARCHIVED: &str = "The archive: done items finished more than `archive_done_days` ago, \
             most recently done first. They stay until `done_item_ttl_days`, then are deleted.";

pub fn tools() -> Vec<Tool> {
    vec![async_tool("list_archived_items", LIST_ARCHIVED, list)]
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListArgs {
    /// Only this area, by id.
    area: Option<String>,
}

async fn list(ctx: Ctx, args: ListArgs) -> AppResult<Vec<WorkItem>> {
    let after = Duration::days(ctx.settings()?.archive_done_days);
    let mut done = archive::select(items::list(&ctx.db).await?, true, Utc::now(), after);
    done.retain(|i| in_area(args.area.as_ref(), &i.area));
    Ok(done)
}
