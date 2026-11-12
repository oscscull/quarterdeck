//! Approving a script schedule. Offered to peers (the user's own app on another machine) but not
//! over MCP, so an agent can write a script but never let it run.

use schemars::JsonSchema;
use serde::Deserialize;

use super::super::ctx::Ctx;
use super::super::tool::{Tool, async_tool};
use crate::config::schedules;
use crate::error::AppResult;
use crate::jobs::approval;

pub fn tools() -> Vec<Tool> {
    vec![async_tool(
        "approve_script",
        "Approves a script schedule as its file stands now, so it may run.",
        approve,
    )]
}

#[derive(Debug, Deserialize, JsonSchema)]
struct IdArgs {
    id: String,
}

async fn approve(ctx: Ctx, args: IdArgs) -> AppResult<()> {
    approval::approve(&ctx.db, &schedules::find(&ctx.config, &args.id)?).await
}
