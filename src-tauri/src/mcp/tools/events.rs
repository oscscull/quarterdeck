use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::super::ctx::Ctx;
use super::super::tool::{NoArgs, Tool, async_tool, tool};
use crate::domain::fired::{self, CUSTOM, Fired, KINDS, Kind};
use crate::error::{AppError, AppResult};
use crate::peers;
use crate::store;

const LIST_EVENT_KINDS: &str = "Events a schedule's trigger can wait for, with the fields its `when` can \
             filter on. Every event also has `area` and `subject` (the item or schedule it's about). \
             Any name starting `custom.` works too, fired with emit_event.";
const EMIT_EVENT: &str = "Fires a custom event, starting the schedules in that area whose triggers wait for \
             it, e.g. after a deploy: kind \"custom.deployed\", data {\"env\": \"prod\"}. The kind \
             must start with `custom.`.";
const RECENT_EVENTS: &str = "The latest events and which schedules each started, newest first.";

/// How many events `list_events` returns.
const RECENT: u32 = 30;

pub fn tools() -> Vec<Tool> {
    vec![
        tool("list_event_kinds", LIST_EVENT_KINDS, kinds),
        async_tool("emit_event", EMIT_EVENT, emit),
        async_tool("list_events", RECENT_EVENTS, recent),
    ]
}

#[derive(Debug, Serialize, JsonSchema)]
struct Kinds {
    kinds: Vec<Kind>,
    custom_prefix: &'static str,
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every tool handler returns AppResult"
)]
fn kinds(_: &Ctx, _: NoArgs) -> AppResult<Kinds> {
    Ok(Kinds {
        kinds: KINDS.to_vec(),
        custom_prefix: CUSTOM,
    })
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct EmitArgs {
    /// e.g. "custom.deployed".
    kind: String,
    /// Area id; the machine that runs it starts the schedules.
    area: String,
    /// What it's about, e.g. an item id. Runs on one subject never overlap.
    #[serde(default)]
    subject: String,
    /// Fields for triggers to filter on.
    #[serde(default)]
    data: BTreeMap<String, String>,
    /// One line telling the run what happened.
    #[serde(default)]
    about: String,
}

async fn emit(ctx: Ctx, args: EmitArgs) -> AppResult<()> {
    if !args.kind.starts_with(CUSTOM) || !fired::known(&args.kind) {
        return Err(AppError::BadInput(format!(
            "custom events start with {CUSTOM}, e.g. {CUSTOM}deployed"
        )));
    }
    let settings = ctx.settings()?;
    let area = ctx.area(&args.area)?;
    let owner = settings.node_for(&area).to_owned();
    if owner != settings.node
        && let Some(peer) = settings.sync.peer(&owner).cloned()
    {
        let args = serde_json::to_value(&args)?;
        tokio::task::spawn_blocking(move || peers::client::call(&peer, "emit_event", &args))
            .await
            .map_err(|e| AppError::Desktop(e.to_string()))??;
        return Ok(());
    }
    let about = if args.about.is_empty() {
        format!("{} was fired", args.kind)
    } else {
        args.about
    };
    let mut event = Fired::new(&args.kind, &area, &args.subject, about);
    event.data = args.data;
    store::fired::emit(&ctx.db, &event).await
}

async fn recent(ctx: Ctx, _: NoArgs) -> AppResult<Vec<Fired>> {
    store::fired::recent(&ctx.db, RECENT).await
}
