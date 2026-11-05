//! A peer saying where it is. Offered to peers only: the server adds the address the call came
//! from, so a machine whose address changed is found again as soon as it reaches this one.

use schemars::JsonSchema;
use serde::Deserialize;

use super::super::ctx::Ctx;
use super::super::tool::{Tool, async_tool};
use crate::error::AppResult;
use crate::peers::addresses;

pub fn tools() -> Vec<Tool> {
    vec![async_tool(
        "hello",
        "Used by other Quarterdeck machines to say where they can be reached.",
        hello,
    )]
}

#[derive(Debug, Deserialize, JsonSchema)]
struct HelloArgs {
    /// The sender's machine name.
    from: String,
    /// URLs the sender gives for itself.
    #[serde(default)]
    urls: Vec<String>,
    /// The URL the call came from, filled in by this machine's server.
    #[serde(default)]
    seen: Option<String>,
}

async fn hello(ctx: Ctx, args: HelloArgs) -> AppResult<()> {
    let urls = args.seen.into_iter().chain(args.urls).collect();
    addresses::learn(&ctx.db, &args.from, urls).await
}
