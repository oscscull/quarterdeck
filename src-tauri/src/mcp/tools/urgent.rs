use chrono::Utc;
use schemars::JsonSchema;
use serde::Deserialize;

use super::super::ctx::Ctx;
use super::super::tool::{NoArgs, Tool, async_tool};
use crate::domain::urgent::Urgent;
use crate::error::AppResult;
use crate::retry;
use crate::store::urgent;

pub fn tools() -> Vec<Tool> {
    vec![
        async_tool(
            "list_urgent",
            "Open urgent items: things that went wrong or can't wait.",
            list,
        ),
        async_tool(
            "raise_urgent",
            "Flags something that can't wait, shown as a red pill on Today with a notification. Use \
             sparingly: only for things that went wrong or need the user soon. Most things are asks.",
            raise,
        ),
        async_tool(
            "dismiss_urgent",
            "Clears an urgent item once it's dealt with.",
            dismiss,
        ),
        async_tool(
            "retry_urgent",
            "Retries the failed agent run an urgent item is about (one with a run_id) and clears it: \
             the item goes back to the agent, whose next prompt is that run's, asked again.",
            retry,
        ),
    ]
}

async fn list(ctx: Ctx, _: NoArgs) -> AppResult<Vec<Urgent>> {
    urgent::list(&ctx.db).await
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RaiseArgs {
    title: String,
    #[serde(default)]
    detail: String,
    /// Area id.
    area: String,
    /// Set when it's about a schedule, so the user can re-run it in one click.
    schedule_id: Option<String>,
}

async fn raise(ctx: Ctx, args: RaiseArgs) -> AppResult<()> {
    let item = Urgent {
        id: 0,
        title: args.title,
        detail: args.detail,
        area: ctx.area(&args.area)?,
        raised_at: Utc::now(),
        schedule_id: args.schedule_id,
        run_id: None,
    };
    urgent::raise(&ctx.db, &item).await?;
    Ok(())
}

#[derive(Debug, Deserialize, JsonSchema)]
struct IdArgs {
    id: i64,
}

async fn dismiss(ctx: Ctx, args: IdArgs) -> AppResult<()> {
    urgent::dismiss(&ctx.db, args.id).await
}

async fn retry(ctx: Ctx, args: IdArgs) -> AppResult<()> {
    retry::request(&ctx.db, args.id, &ctx.settings()?.turns).await
}
