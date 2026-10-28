use chrono::Utc;
use schemars::JsonSchema;
use serde::Deserialize;

use super::super::ctx::Ctx;
use super::super::tool::{Tool, async_tool};
use super::{in_area, time};
use crate::domain::event::Change;
use crate::domain::item::{ItemInput, Stage, Status, WorkItem};
use crate::domain::trigger::OnEvent;
use crate::domain::turn::Turn;
use crate::error::AppResult;
use crate::store::{asks, events, items};
use crate::turns::Clock;
use crate::work;

const LIST_ITEMS: &str = "Work items (tickets, PRs, issues, personal projects) in priority order, highest \
             first. `parked` lists only parked work (on a later turn).";
const GET_ITEM: &str = "One item with its open asks and its history.";
const CREATE_ITEM: &str = "Adds a work item to the bottom of the priorities. Use the ticket key as `key` when there \
             is one (e.g. JD-104). Up next (the default) is unstarted backlog: nobody's turn, off \
             Today. Work made in progress or in review starts as the user's turn; take it on with \
             claim_item or hand it on with pass_turn. Returns the item's id.";
const UPDATE_ITEM: &str = "Changes an item. Only the fields you pass change. Keep `state_label` short and current, \
             e.g. \"waiting on review from Sam\". Set `status` to what it's waiting on, or \
             `clear_status` when it's moving again. Whose turn it is changes only through \
             pass_turn. Started work can't go back to up next: park it or mark it done.";
const DELETE_ITEM: &str = "Deletes an item with its asks and history.";
const MOVE_ITEM: &str = "Moves an item to a position in the priorities, 1 being the top.";
const PARK_ITEM: &str = "Parks started work until something happens: a time (`until`, RFC 3339), \
             events (`wake`, triggers as in schedules, e.g. {\"on\": \"item.stage\", \"when\": \
             {\"item\": \"app#9\", \"stage\": \"done\"}}), or both, whichever comes first. With only \
             events, the configured later length is the backstop. Use it for any fixed wait (e.g. \
             \"14 days for traffic to reach 0\"): don't leave such work on the user's or others' turn. \
             It leaves Today for the Parked list (with a countdown) until then, then comes back as \
             the user's turn, or the agent's with `then: agent`. `why` says what it waits for and what \
             to do once it's back (required for the agent); it's repeated when it wakes. Nothing \
             parks for good: close work nobody will pick up (update_item stage done). Unstarted work \
             can't be parked; it's already off Today. `parked: false` brings it back now.";
const ADD_NOTE: &str = "Adds a line to an item's history, e.g. \"opened PR #821\". Use for progress worth \
             remembering; the user sees it in the item's history.";

pub fn tools() -> Vec<Tool> {
    vec![
        async_tool("list_items", LIST_ITEMS, list),
        async_tool("get_item", GET_ITEM, get),
        async_tool("create_item", CREATE_ITEM, create),
        async_tool("update_item", UPDATE_ITEM, update),
        async_tool("delete_item", DELETE_ITEM, delete),
        async_tool("move_item", MOVE_ITEM, move_to),
        async_tool("park_item", PARK_ITEM, park),
        async_tool("add_note", ADD_NOTE, note),
    ]
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListArgs {
    /// Only this area, by id.
    area: Option<String>,
    /// Only this stage.
    stage: Option<Stage>,
    /// Include done items. Defaults to false.
    #[serde(default)]
    include_done: bool,
    /// List the parked items instead. Defaults to false.
    #[serde(default)]
    parked: bool,
}

async fn list(ctx: Ctx, args: ListArgs) -> AppResult<Vec<WorkItem>> {
    let mut all = if args.parked {
        crate::park::list(&ctx.db).await?
    } else {
        items::list(&ctx.db).await?
    };
    all.retain(|i| {
        in_area(args.area.as_ref(), &i.area)
            && args.stage.is_none_or(|s| s == i.stage())
            && (args.include_done || i.stage() != Stage::Done)
    });
    Ok(all)
}

#[derive(Debug, Deserialize, JsonSchema)]
struct IdArgs {
    id: String,
}

async fn get(ctx: Ctx, args: IdArgs) -> AppResult<serde_json::Value> {
    Ok(serde_json::json!({
        "item": items::get(&ctx.db, &args.id).await?,
        "asks": asks::for_item(&ctx.db, &args.id).await?,
        "history": events::for_item(&ctx.db, &args.id).await?,
    }))
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateArgs {
    title: String,
    /// Area id, e.g. "work".
    area: String,
    /// Ticket key to use as the id. A short id is made up if left out.
    key: Option<String>,
    stage: Option<Stage>,
    /// What it's waiting on, if anything.
    status: Option<Status>,
    /// Where it's at, in a few words.
    state_label: Option<String>,
    /// URL of the ticket, PR or doc.
    link: Option<String>,
}

async fn create(ctx: Ctx, args: CreateArgs) -> AppResult<String> {
    let input = ItemInput {
        id: args.key.unwrap_or_default(),
        title: args.title,
        state_label: args.state_label.unwrap_or_default(),
        stage: args.stage.unwrap_or(Stage::UpNext),
        status: args.status,
        area: ctx.area(&args.area)?,
        link: args.link,
    };
    work::save(&ctx.db, input, Utc::now()).await
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateArgs {
    id: String,
    title: Option<String>,
    area: Option<String>,
    stage: Option<Stage>,
    status: Option<Status>,
    /// Set true to say it's no longer waiting on anything.
    #[serde(default)]
    clear_status: bool,
    state_label: Option<String>,
    link: Option<String>,
}

async fn update(ctx: Ctx, args: UpdateArgs) -> AppResult<String> {
    let area = args.area.as_deref().map(|a| ctx.area(a)).transpose()?;
    let old = items::get(&ctx.db, &args.id).await?;
    let stage = args.stage.unwrap_or_else(|| old.stage());
    let input = ItemInput {
        stage,
        status: if args.clear_status {
            None
        } else {
            args.status.or(old.status)
        },
        id: old.id,
        title: args.title.unwrap_or(old.title),
        state_label: args.state_label.unwrap_or(old.state_label),
        area: area.unwrap_or(old.area),
        link: args.link.or(old.link),
    };
    work::save(&ctx.db, input, Utc::now()).await
}

async fn delete(ctx: Ctx, args: IdArgs) -> AppResult<()> {
    items::delete(&ctx.db, &args.id).await
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MoveArgs {
    id: String,
    /// 1 is the top of the priorities.
    position: usize,
}

async fn move_to(ctx: Ctx, args: MoveArgs) -> AppResult<()> {
    let mut ids: Vec<String> = items::list(&ctx.db)
        .await?
        .into_iter()
        .map(|i| i.id)
        .filter(|i| *i != args.id)
        .collect();
    items::get(&ctx.db, &args.id).await?;
    ids.insert(
        args.position.saturating_sub(1).min(ids.len()),
        args.id.clone(),
    );
    items::reorder(&ctx.db, &ids).await
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ParkArgs {
    id: String,
    /// False brings it back now. Defaults to true.
    #[serde(default = "yes")]
    parked: bool,
    /// When it comes back, as RFC 3339.
    until: Option<String>,
    /// Events that bring it back sooner.
    #[serde(default)]
    wake: Vec<OnEvent>,
    /// What it waits for and what to do once it's back. Noted in its history and repeated when it
    /// wakes.
    why: Option<String>,
    /// Who picks it up once it's back: `you` (the default) or `agent`.
    then: Option<Turn>,
}

const fn yes() -> bool {
    true
}

async fn park(ctx: Ctx, args: ParkArgs) -> AppResult<()> {
    let limits = ctx.settings()?.turns;
    let clock = Clock::now(&limits);
    if !args.parked {
        return crate::park::unpark(&ctx.db, &args.id, clock).await;
    }
    let parking = crate::park::Parking {
        until: args.until.as_deref().map(time).transpose()?,
        wake: args.wake,
        why: args.why,
        then: args.then,
    };
    crate::park::park(&ctx.db, &args.id, parking, clock).await
}

#[derive(Debug, Deserialize, JsonSchema)]
struct NoteArgs {
    item_id: String,
    text: String,
}

async fn note(ctx: Ctx, args: NoteArgs) -> AppResult<()> {
    items::get(&ctx.db, &args.item_id).await?;
    events::record(
        &ctx.db,
        &args.item_id,
        Utc::now(),
        &Change::Note { text: args.text },
    )
    .await
}
