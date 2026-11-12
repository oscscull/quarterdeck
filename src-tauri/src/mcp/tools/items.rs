use chrono::Utc;
use schemars::JsonSchema;
use serde::Deserialize;

use super::super::ctx::Ctx;
use super::super::tool::{Tool, async_tool};
use super::{in_area, time};
use crate::domain::event::{Change, RecordTag};
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
             is one (e.g. JD-104). Every new item is the user's turn, on Today: up next (the \
             default) asks them to start it, park it or close it. If it can start now, start it: \
             take it on with claim_item or pass_turn to agent. If it can't start yet (it needs \
             something that doesn't exist yet, or must wait for a date), park_item it saying why. \
             Never leave work that can start sitting, and never park work only because it isn't \
             due yet. Returns the item's id.";
const UPDATE_ITEM: &str = "Changes an item. Only the fields you pass change. `project` puts it in a project (\"\" takes it out). Keep `state_label` short and current, \
             e.g. \"waiting on review from Sam\". Set `status` to what it's waiting on, or \
             `clear_status` when it's moving again. Whose turn it is changes only through \
             pass_turn, and moving it between stages keeps its turn.";
const DELETE_ITEM: &str = "Deletes an item with its asks and history.";
const MOVE_ITEM: &str = "Moves an item to a position in the priorities, 1 being the top.";
const PARK_ITEM: &str = "Parks work, started or not, that can't start or go on yet. Never for work \
             that merely isn't due: work that can start, starts. `why` is required: what stops it \
             now and what to do once it can. Either until a date (`until`, RFC 3339; e.g. \
             \"restarts 1 Dec\", \"14 days for traffic to reach 0\"), when it comes back as the \
             user's turn, or the agent's with `then: agent`; or, with no `until`, until something \
             happens: `wake` events bring it back at once (triggers as in schedules, e.g. \
             {\"on\": \"item.stage\", \"when\": {\"item\": \"app#9\", \"stage\": \"done\"}}), \
             and the agent checks `why` at least daily regardless, starting it or parking it \
             again. It leaves Today for the Parked list (with a countdown) meanwhile. Nothing parks \
             for good: close work nobody will pick up (update_item stage done). `parked: false` \
             brings it back now.";
const ADD_RECORD: &str = "Adds a record to an item: one plain sentence a person will read later, about \
             something that mattered. `tag`: why (why the work exists: the problem or need), change (a \
             change of direction), blocked (blocked or unblocked by something outside), correction (the \
             user corrected the picture), decision (the user decided something) or delivered (what the \
             finished work changed for the user). Explain the situation: what was wrong or needed and \
             what changed, never a vague verdict like \"improved things\". No routine progress, commit \
             hashes or test counts: stage, turn and label changes are kept as trace already. The Briefing \
             is written from these.";

pub fn tools() -> Vec<Tool> {
    vec![
        async_tool("list_items", LIST_ITEMS, list),
        async_tool("get_item", GET_ITEM, get),
        async_tool("create_item", CREATE_ITEM, create),
        async_tool("update_item", UPDATE_ITEM, update),
        async_tool("delete_item", DELETE_ITEM, delete),
        async_tool("move_item", MOVE_ITEM, move_to),
        async_tool("park_item", PARK_ITEM, park),
        async_tool("add_record", ADD_RECORD, record),
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
    /// Only items in this project, by id.
    project: Option<String>,
}

async fn list(ctx: Ctx, args: ListArgs) -> AppResult<Vec<WorkItem>> {
    let mut all = if args.parked {
        crate::park::list(&ctx.db).await?
    } else {
        items::list(&ctx.db).await?
    };
    let membership = items::projects(&ctx.db).await?;
    all.retain(|i| {
        in_area(args.area.as_ref(), &i.area)
            && args.stage.is_none_or(|s| s == i.stage())
            && (args.include_done || i.stage() != Stage::Done)
            && args
                .project
                .as_ref()
                .is_none_or(|p| membership.get(&i.id) == Some(p))
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
        "project": items::projects(&ctx.db).await?.remove(&args.id),
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
    /// The project it belongs to, by id.
    project: Option<String>,
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
    let id = work::save(&ctx.db, input, Utc::now()).await?;
    if let Some(project) = args.project {
        super::projects::assign(&ctx, &id, &project).await?;
    }
    Ok(id)
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
    /// The project it belongs to, by id; "" takes it out of its project.
    project: Option<String>,
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
    let id = work::save(&ctx.db, input, Utc::now()).await?;
    if let Some(project) = args.project {
        super::projects::assign(&ctx, &id, &project).await?;
    }
    Ok(id)
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
    /// Required: what stops it now and what to do once it can. Noted in its history and repeated
    /// when it wakes.
    why: Option<String>,
    /// Who picks it up when it's back on its date: `you` (the default) or `agent`. Without a date
    /// the agent checks it.
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
struct RecordArgs {
    item_id: String,
    tag: RecordTag,
    /// One plain sentence.
    text: String,
}

async fn record(ctx: Ctx, args: RecordArgs) -> AppResult<()> {
    items::get(&ctx.db, &args.item_id).await?;
    let change = Change::Record {
        tag: args.tag,
        text: args.text,
    };
    events::record(&ctx.db, &args.item_id, Utc::now(), &change).await
}
