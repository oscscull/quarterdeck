use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::super::ctx::Ctx;
use super::super::tool::{NoArgs, Tool, async_tool};
use crate::config::edit;
use crate::domain::item::Stage;
use crate::error::{AppError, AppResult};
use crate::queue::{Next, next};
use crate::store::items;

const SET: &str = "Sets the items Claude works on one at a time, in order, by item id (replacing \
the list). The first not yet in review or done starts when it's up next, as if the user told Claude \
to work on it (parked or waiting up-next work is passed over until it can start); the next waits until it's in review or done. An item linking a GitHub issue \
(https://github.com/<owner>/<repo>/issues/<n>) in a repo with a checkout (set_repo) gets its own \
branch and worktree, and Quarterdeck pushes it and opens a PR closing the issue when Claude finishes. \
An empty list clears the queue.";

pub fn tools() -> Vec<Tool> {
    vec![
        async_tool(
            "get_queue",
            "The items Claude works on one at a time, in order, with each one's stage, and which \
             is current.",
            get,
        ),
        async_tool("set_queue", SET, set),
    ]
}

#[derive(Debug, Serialize)]
struct Entry {
    id: String,
    /// None when no such item exists (any more).
    stage: Option<Stage>,
    state_label: String,
}

#[derive(Debug, Serialize)]
struct Queue {
    items: Vec<Entry>,
    /// The item being worked on or about to start.
    current: Option<String>,
}

async fn get(ctx: Ctx, _: NoArgs) -> AppResult<Queue> {
    let ids = ctx.settings()?.queue.items;
    let all = items::list(&ctx.db).await?;
    let current = match next(&ids, &all) {
        Next::Start(i) | Next::Wait(i) => Some(i.id.clone()),
        Next::Finished => None,
    };
    let entries = ids
        .into_iter()
        .map(|id| {
            let item = all.iter().find(|i| i.id == id);
            Entry {
                stage: item.map(crate::domain::item::WorkItem::stage),
                state_label: item.map(|i| i.state_label.clone()).unwrap_or_default(),
                id,
            }
        })
        .collect();
    Ok(Queue {
        items: entries,
        current,
    })
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SetArgs {
    /// Item ids, first to work on first.
    items: Vec<String>,
}

async fn set(ctx: Ctx, args: SetArgs) -> AppResult<()> {
    let mut unknown = Vec::new();
    for id in &args.items {
        if items::find(&ctx.db, id).await?.is_none() {
            unknown.push(id.as_str());
        }
    }
    if !unknown.is_empty() {
        return Err(AppError::BadInput(format!(
            "no such item: {}",
            unknown.join(", ")
        )));
    }
    edit::set_queue(&ctx.config, &args.items)
}
