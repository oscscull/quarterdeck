use chrono::{Local, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::super::ctx::Ctx;
use super::super::tool::{NoArgs, Tool, async_tool, tool};
use super::time;
use crate::config::{pause, schedules};
use crate::cron::explain::{Explanation, explain};
use crate::domain::schedule::Schedule;
use crate::domain::script::Script;
use crate::domain::trigger::Trigger;
use crate::error::AppResult;
use crate::jobs::approval;
use crate::peers;
use crate::store::run_requests;
use crate::views::agent::{self, AgentView};

const LIST_SCHEDULES: &str = "Every recurring job, with when it runs next, its recent runs, what Claude is running now, \
             and whether everything is paused.";
const CREATE_SCHEDULE: &str = "Creates a recurring job: instructions Quarterdeck runs through Claude, or, with \
             `script`, a shell script it runs itself (no tokens; for anything that needs no judgement). \
             `triggers` says when: any of \
             {\"type\": \"cron\", \"expr\": \"30 8 * * 1-5\"} (minute hour day month weekday) or \
             {\"type\": \"event\", \"on\": \"item.turn\", \"when\": {\"to\": \"agent\"}, \"cooldown\": 30} \
             (see list_event_kinds; `when` filters on the event's fields, `*` is a wildcard; \
             `cooldown` is minutes between runs on the same subject). Prefer an event when one fits: \
             events cost nothing until they happen. Runs can only read unless `allowed_tools` lists \
             Claude Code tools they may use, e.g. [\"Bash(gh pr comment:*)\"]. Returns the schedule's id.";
const UPDATE_SCHEDULE: &str = "Changes a schedule. Only the fields you pass change.";
const SET_SCHEDULE_ENABLED: &str = "Turns one schedule on or off.";
const RUN_SCHEDULE: &str = "Runs a schedule now (within 30 seconds), even if it's paused.";
const PAUSE_ALL: &str = "Pauses every schedule, e.g. while the user is on holiday. Nothing runs and missed runs \
             aren't flagged. `until` is an RFC 3339 time; leave it out to pause until resumed.";
const EXPLAIN_CRON: &str = "Explains a cron expression in plain English with its next run times.";

pub fn tools() -> Vec<Tool> {
    vec![
        async_tool("list_schedules", LIST_SCHEDULES, list),
        tool("create_schedule", CREATE_SCHEDULE, create),
        tool("update_schedule", UPDATE_SCHEDULE, update),
        async_tool("delete_schedule", "Deletes a schedule.", delete),
        tool("set_schedule_enabled", SET_SCHEDULE_ENABLED, set_enabled),
        async_tool("run_schedule", RUN_SCHEDULE, run),
        tool("pause_all", PAUSE_ALL, pause_all),
        tool("resume_all", "Ends a pause.", resume_all),
        tool("explain_cron", EXPLAIN_CRON, explain_cron),
    ]
}

async fn list(ctx: Ctx, _: NoArgs) -> AppResult<AgentView> {
    let loaded = schedules::list(&ctx.config)?;
    let paused = pause::current(&ctx.config, Utc::now())?;
    let limits = ctx.settings()?.turns;
    agent::build(&ctx.db, loaded, paused, &limits).await
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct CreateArgs {
    name: String,
    /// Area id; the area decides which machine runs it.
    area: String,
    /// What Claude should do each time, written to Claude; or, with `script`, the shell script.
    instructions: String,
    /// Makes it a script schedule: `instructions` runs in the user's login shell, with no Claude.
    /// A script made or changed through a tool doesn't run until the user approves it in the app.
    script: Option<Script>,
    /// When it runs; any one of them starts it. Empty means only when run by hand.
    #[serde(default)]
    triggers: Vec<Trigger>,
    #[serde(default)]
    allowed_tools: Vec<String>,
    /// The user's own words, if they asked for it.
    #[serde(default)]
    request: String,
}

fn create(ctx: &Ctx, args: CreateArgs) -> AppResult<String> {
    // A schedule lives on the machine that runs its area, so hand it over when that's a peer.
    let settings = ctx.settings()?;
    let owner = settings.node_for(&ctx.area(&args.area)?).to_owned();
    if owner != settings.node
        && let Some(peer) = settings.sync.peer(&owner)
    {
        let id = peers::client::call(peer, "create_schedule", &serde_json::to_value(&args)?)?;
        return Ok(id.as_str().unwrap_or_default().to_owned());
    }
    let schedule = Schedule {
        id: schedules::free_id(&ctx.config, &schedules::slug(&args.name)),
        name: args.name,
        prompt: args.instructions,
        script: args.script,
        triggers: args.triggers.into_iter().map(Trigger::normalised).collect(),
        area: ctx.area(&args.area)?,
        enabled: true,
        request: args.request,
        allowed_tools: args.allowed_tools,
    };
    schedules::check(&schedule)?;
    schedules::save(&ctx.config, &schedule)?;
    Ok(schedule.id)
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateArgs {
    id: String,
    name: Option<String>,
    area: Option<String>,
    instructions: Option<String>,
    /// Replaces every trigger.
    triggers: Option<Vec<Trigger>>,
    /// Makes it a script schedule with these settings.
    script: Option<Script>,
    /// Makes a script schedule a Claude one again.
    #[serde(default)]
    remove_script: bool,
    allowed_tools: Option<Vec<String>>,
}

fn update(ctx: &Ctx, args: UpdateArgs) -> AppResult<()> {
    let old = schedules::find(&ctx.config, &args.id)?;
    let schedule = Schedule {
        triggers: args
            .triggers
            .map(|t| t.into_iter().map(Trigger::normalised).collect())
            .unwrap_or(old.triggers),
        area: args
            .area
            .as_deref()
            .map(|a| ctx.area(a))
            .transpose()?
            .unwrap_or(old.area),
        name: args.name.unwrap_or(old.name),
        prompt: args.instructions.unwrap_or(old.prompt),
        script: if args.remove_script {
            None
        } else {
            args.script.or(old.script)
        },
        allowed_tools: args.allowed_tools.unwrap_or(old.allowed_tools),
        ..old
    };
    schedules::check(&schedule)?;
    schedules::save(&ctx.config, &schedule)
}

#[derive(Debug, Deserialize, JsonSchema)]
struct IdArgs {
    id: String,
}

async fn delete(ctx: Ctx, args: IdArgs) -> AppResult<()> {
    schedules::delete(&ctx.config, &args.id)?;
    approval::forget(&ctx.db, &args.id).await
}

#[derive(Debug, Deserialize, JsonSchema)]
struct EnabledArgs {
    id: String,
    enabled: bool,
}

fn set_enabled(ctx: &Ctx, args: EnabledArgs) -> AppResult<()> {
    let mut schedule = schedules::find(&ctx.config, &args.id)?;
    schedule.enabled = args.enabled;
    schedules::save(&ctx.config, &schedule)
}

/// Runs on the machine that owns the schedule's area, moving it there first if it's held here.
async fn run(ctx: Ctx, args: IdArgs) -> AppResult<()> {
    let schedule = schedules::find(&ctx.config, &args.id)?;
    let settings = ctx.settings()?;
    if peers::handoff::run_elsewhere(&ctx.db, &ctx.config, &settings, schedule).await? {
        return Ok(());
    }
    run_requests::add(&ctx.db, &args.id).await
}

#[derive(Debug, Deserialize, JsonSchema)]
struct PauseArgs {
    /// RFC 3339; leave out to pause until resumed.
    until: Option<String>,
}

fn pause_all(ctx: &Ctx, args: PauseArgs) -> AppResult<()> {
    pause::pause(&ctx.config, args.until.as_deref().map(time).transpose()?)
}

fn resume_all(ctx: &Ctx, _: NoArgs) -> AppResult<()> {
    pause::resume(&ctx.config)
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ExplainArgs {
    expr: String,
}

fn explain_cron(_: &Ctx, args: ExplainArgs) -> AppResult<Explanation> {
    explain(&args.expr, Local::now(), 3)
}
