use super::super::ctx::Ctx;
use super::super::tool::{NoArgs, Tool, async_tool};
use crate::domain::outage::Outage;
use crate::error::AppResult;
use crate::scheduler::outage;

pub fn tools() -> Vec<Tool> {
    vec![async_tool(
        "check_agent",
        "Checks now whether Quarterdeck's agent can run (`claude` found and logged in, network up) \
         instead of waiting for the next automatic check. Returns the outage still going on, or \
         null when the agent can run, which lets its waiting work carry on.",
        check,
    )]
}

async fn check(ctx: Ctx, _: NoArgs) -> AppResult<Option<Outage>> {
    outage::check_now(&ctx.db, &ctx.settings()?).await
}
