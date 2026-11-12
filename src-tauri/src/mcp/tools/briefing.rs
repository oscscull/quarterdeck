use chrono::Utc;
use schemars::JsonSchema;
use serde::Deserialize;

use super::super::ctx::Ctx;
use super::super::tool::{Tool, async_tool};
use crate::briefing::inputs::{self, Inputs};
use crate::briefing::view::{self, BriefingView};
use crate::briefing::{due, write};
use crate::domain::project::Briefing;
use crate::error::AppResult;

const GET: &str = "The Briefing: each area's combined account of the work, each project's own, and the \
references they cite. Citations are `[ref:<id>]` markers.";
const INPUTS: &str = "What an area's briefing is written from: its projects, its open and just-finished \
items with their records, its references, and the current briefing. For the briefing run.";
const WRITE: &str = "Saves an area's rewritten briefing: the combined one, and one per project. Each has \
`done`, `now` and `next` lists of paragraphs ({project, text}); cite references as `[ref:<id>]`. For the \
briefing run (prompts/briefing.md).";
const REFRESH: &str = "Asks for an area's briefing to be rewritten soon, e.g. after the user corrected it \
and any agreed changes are made.";

/// Only peers get this: the user's own app on another machine, correcting a briefing this one holds.
pub fn peer_tools() -> Vec<Tool> {
    vec![async_tool(
        "correct_briefing",
        "Answers one message from the Briefing's correction box.",
        correct,
    )]
}

async fn correct(ctx: Ctx, args: crate::briefing::correct::Correction) -> AppResult<String> {
    crate::briefing::correct::reply(&ctx.db, &ctx.config, &ctx.settings()?, &args).await
}

pub fn tools() -> Vec<Tool> {
    vec![
        async_tool("get_briefing", GET, get),
        async_tool("get_briefing_inputs", INPUTS, get_inputs),
        async_tool("write_briefing", WRITE, write_it),
        async_tool("refresh_briefing", REFRESH, refresh),
    ]
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetArgs {
    /// Only this area, by id.
    area: Option<String>,
}

async fn get(ctx: Ctx, args: GetArgs) -> AppResult<BriefingView> {
    view::build(&ctx.db, &ctx.settings()?, args.area.as_deref()).await
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AreaArgs {
    area: String,
}

async fn get_inputs(ctx: Ctx, args: AreaArgs) -> AppResult<Inputs> {
    let days = ctx.settings()?.briefing.just_done_days;
    inputs::gather(&ctx.db, &ctx.area(&args.area)?, days, Utc::now()).await
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ProjectBriefing {
    /// The project's id.
    id: String,
    briefing: Briefing,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct WriteArgs {
    area: String,
    /// The area's combined briefing; every paragraph names its project.
    combined: Briefing,
    /// Each active project's own briefing.
    #[serde(default)]
    projects: Vec<ProjectBriefing>,
}

async fn write_it(ctx: Ctx, args: WriteArgs) -> AppResult<()> {
    let rewrite = write::Rewrite {
        combined: args.combined,
        projects: args
            .projects
            .into_iter()
            .map(|p| (p.id, p.briefing))
            .collect(),
    };
    write::save(&ctx.db, &ctx.area(&args.area)?, &rewrite, Utc::now()).await
}

async fn refresh(ctx: Ctx, args: AreaArgs) -> AppResult<()> {
    due::request(&ctx.db, &ctx.area(&args.area)?).await
}
