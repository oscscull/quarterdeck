use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::super::ctx::Ctx;
use super::super::tool::{NoArgs, Tool, async_tool, tool};
use super::in_area;
use crate::config::settings::Area;
use crate::error::AppResult;
use crate::views::today::{self, TodayView};

pub fn tools() -> Vec<Tool> {
    vec![
        async_tool(
            "get_overview",
            "Everything on the user's Today screen: upcoming meetings, the asks that need them on \
             their highest-ranked work, to-dos, GitHub issues waiting to be triaged, urgent items, \
             recent updates, and `outage` while the agent can't run at all. Start here.",
            overview,
        ),
        tool(
            "list_areas",
            "The user's areas of life (e.g. work, projects, life) and which machine runs each. Every \
             item, to-do, meeting and schedule belongs to one area, by id.",
            areas,
        ),
        tool(
            "get_config_info",
            "Where Quarterdeck's editable files live, for changes the tools don't cover, such as the \
             prompts Quarterdeck's own Claude uses. Prefer the tools: set_setting covers limits, TTLs, \
             sync intervals, alerts and updates.",
            config_info,
        ),
    ]
}

#[derive(Debug, Deserialize, JsonSchema)]
struct OverviewArgs {
    /// Only show one area, by id. Leave out for everything.
    area: Option<String>,
}

async fn overview(ctx: Ctx, args: OverviewArgs) -> AppResult<TodayView> {
    let settings = ctx.settings()?;
    let limit = settings.priority_limit;
    let mut view = today::build(&ctx.db, limit, Utc::now()).await?;
    let keep = |a: &str| in_area(args.area.as_ref(), a);
    view.upcoming.retain(|c| keep(&c.area));
    view.priorities.retain(|p| keep(&p.area));
    view.priorities.truncate(limit);
    view.todos.retain(|t| keep(&t.area));
    view.triage.retain(|t| keep(&t.area));
    view.urgent.retain(|u| keep(&u.area));
    view.activity.retain(|a| keep(&a.area));
    Ok(view)
}

#[derive(Debug, Serialize)]
struct Areas {
    this_machine: String,
    areas: Vec<Area>,
}

fn areas(ctx: &Ctx, _: NoArgs) -> AppResult<Areas> {
    let settings = ctx.settings()?;
    Ok(Areas {
        this_machine: settings.node,
        areas: settings.areas,
    })
}

#[derive(Debug, Serialize)]
struct ConfigInfo {
    folder: String,
    files: [&'static str; 5],
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every tool handler returns AppResult"
)]
fn config_info(ctx: &Ctx, _: NoArgs) -> AppResult<ConfigInfo> {
    Ok(ConfigInfo {
        folder: ctx.config.root().display().to_string(),
        files: [
            "quarterdeck.toml: this machine's name, areas ([[areas]] with id, name, color, node), connectors, limits and TTLs",
            "schedules/<id>.md: one per recurring job (prefer the schedule tools)",
            "prompts/*.md: prompts Quarterdeck's built-in Claude uses",
            "inbox/<name>.toml: drop title/detail/area to raise an urgent item (or use raise_urgent)",
            "pause.toml: present while all schedules are paused (prefer pause_all/resume_all)",
        ],
    })
}
