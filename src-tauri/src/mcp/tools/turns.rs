use schemars::JsonSchema;
use serde::Deserialize;

use super::super::ctx::Ctx;
use super::super::tool::{Tool, async_tool, tool};
use super::time;
use crate::config::edit;
use crate::config::turns::TurnChange;
use crate::domain::turn::Turn;
use crate::error::{AppError, AppResult};
use crate::store::items;
use crate::turns::{self, Clock, Pass};

const PASS: &str = "Hands an item's turn on, saying what you did. Every open item, started or not, \
is someone's turn with a clock on it, so nothing waits forever; giving the agent the turn on an \
up-next item starts it. `to`: `agent` to carry on yourself (Quarterdeck prompts you again; the \
clock doesn't restart), `others` when it waits on reviewers, CI or another team (say who in \
`waiting_on`; it's checked again within a day: by `then`, `agent` by default, or `you`), `you` \
when the user must decide or act. Neither the agent's nor others' turn can last over a day. To put \
work off until it can start, use park_item. `did` is required: \
one line on what you did or why you stopped. Passing to `you` also needs `need`: one line on \
exactly what you need from them, e.g. \"Close #59 and decide the v1 version (#73)\". Today shows \
it as the headline.";
const LIMITS: &str = "Sets how long each turn lasts before Quarterdeck acts: agent_hours (then back \
to the user), you_remind_hours (between reminders), others_hours (then to others_then), \
later_hours (how often work parked until something happens is checked), others_then (\"agent\" \
to check or \"you\"); agent_hours, others_hours and later_hours are at most 24. quiet_minutes (how long an agent session \
outside Quarterdeck may go quiet before it's prompted), and failure_retries (how many times in a \
row a failed run is tried again by itself before it goes to the user). Only the fields you pass change.";

pub fn tools() -> Vec<Tool> {
    vec![
        async_tool("pass_turn", PASS, pass),
        tool("set_turn_limits", LIMITS, limits),
    ]
}

#[derive(Debug, Deserialize, JsonSchema)]
struct PassArgs {
    id: String,
    to: Turn,
    /// What you did, or why you stopped, in one line.
    did: String,
    /// For `others`: who it waits on, e.g. "CI" or reviewers' names.
    waiting_on: Option<String>,
    /// When the turn runs out, as RFC 3339. Defaults to the configured length.
    until: Option<String>,
    /// For `others` or `later`: who gets the turn after, `you` or `agent`.
    then: Option<Turn>,
    /// For `you`: what you need from the user, in one line.
    need: Option<String>,
}

async fn pass(ctx: Ctx, args: PassArgs) -> AppResult<()> {
    if args.did.trim().is_empty() {
        return Err(AppError::BadInput("say what you did in `did`".to_owned()));
    }
    let needs = args.need.as_deref().map(str::trim).unwrap_or_default();
    if args.to == Turn::You && needs.is_empty() {
        return Err(AppError::BadInput(
            "say what you need from the user in `need`".to_owned(),
        ));
    }
    if args.to == Turn::Later {
        return Err(AppError::BadInput(
            "to put it off, use park_item, which says why and when it comes back".to_owned(),
        ));
    }
    if matches!(args.then, Some(Turn::Others | Turn::Later)) {
        return Err(AppError::BadInput(
            "`then` is who acts after the wait: `you` or `agent`".to_owned(),
        ));
    }
    items::get(&ctx.db, &args.id).await?;
    let pass = Pass {
        to: args.to,
        why: args.did,
        need: args.need,
        waiting_on: args.waiting_on,
        until: args.until.as_deref().map(time).transpose()?,
        then: args.then,
        wake: Vec::new(),
    };
    let limits = ctx.settings()?.turns;
    turns::pass(&ctx.db, &args.id, pass, Clock::now(&limits)).await
}

#[derive(Debug, Deserialize, JsonSchema)]
struct LimitArgs {
    agent_hours: Option<i64>,
    you_remind_hours: Option<i64>,
    others_hours: Option<i64>,
    later_hours: Option<i64>,
    others_then: Option<Turn>,
    quiet_minutes: Option<i64>,
    failure_retries: Option<u32>,
}

fn limits(ctx: &Ctx, args: LimitArgs) -> AppResult<()> {
    let change = TurnChange {
        agent_hours: args.agent_hours,
        you_remind_hours: args.you_remind_hours,
        others_hours: args.others_hours,
        later_hours: args.later_hours,
        others_then: args.others_then,
        quiet_minutes: args.quiet_minutes,
        failure_retries: args.failure_retries,
    };
    edit::set_turns(&ctx.config, change)
}
