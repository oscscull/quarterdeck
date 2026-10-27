use chrono::Utc;
use schemars::JsonSchema;
use serde::Deserialize;

use super::super::ctx::Ctx;
use super::super::tool::{Tool, async_tool};
use super::{date, in_area};
use crate::domain::todo::{Todo, link_or_none};
use crate::error::AppResult;
use crate::store::todos;

pub fn tools() -> Vec<Tool> {
    vec![
        async_tool(
            "list_todos",
            "To-dos: things only the user can do, like errands and bookings. Open ones first, soonest due first.",
            list,
        ),
        async_tool(
            "add_todo",
            "Adds a to-do for the user, e.g. \"Make dinner\" due Friday. Use for things they must do \
             themselves; use work items for work you or others will carry.",
            add,
        ),
        async_tool(
            "update_todo",
            "Changes a to-do. Only the fields you pass change.",
            update,
        ),
        async_tool(
            "complete_todo",
            "Ticks a to-do off (or back on with `done: false`).",
            complete,
        ),
        async_tool("delete_todo", "Deletes a to-do.", delete),
    ]
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListArgs {
    /// Only this area, by id.
    area: Option<String>,
    /// Include ticked-off ones. Defaults to false.
    #[serde(default)]
    include_done: bool,
}

async fn list(ctx: Ctx, args: ListArgs) -> AppResult<Vec<Todo>> {
    let mut all = todos::list(&ctx.db).await?;
    all.retain(|t| {
        in_area(args.area.as_ref(), &t.area) && (args.include_done || t.done_at.is_none())
    });
    Ok(all)
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AddArgs {
    title: String,
    /// Area id, e.g. "life".
    area: String,
    /// YYYY-MM-DD
    due: Option<String>,
    /// A page it's about, e.g. the booking form or the email to answer.
    link: Option<String>,
}

async fn add(ctx: Ctx, args: AddArgs) -> AppResult<()> {
    let todo = Todo {
        id: 0,
        title: args.title,
        area: ctx.area(&args.area)?,
        due: args.due.as_deref().map(date).transpose()?,
        link: link_or_none(args.link),
        done_at: None,
    };
    todos::insert(&ctx.db, &todo).await
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateArgs {
    id: i64,
    title: Option<String>,
    area: Option<String>,
    /// YYYY-MM-DD
    due: Option<String>,
    /// Set true to remove the due date.
    #[serde(default)]
    clear_due: bool,
    /// A page it's about. Pass "" to remove it.
    link: Option<String>,
}

async fn update(ctx: Ctx, args: UpdateArgs) -> AppResult<()> {
    let area = args.area.as_deref().map(|a| ctx.area(a)).transpose()?;
    let due = args.due.as_deref().map(date).transpose()?;
    let old = todos::get(&ctx.db, args.id).await?;
    let todo = Todo {
        title: args.title.unwrap_or(old.title),
        area: area.unwrap_or(old.area),
        due: if args.clear_due {
            None
        } else {
            due.or(old.due)
        },
        link: match args.link {
            Some(l) => link_or_none(Some(l)),
            None => old.link,
        },
        ..old
    };
    todos::update(&ctx.db, &todo).await
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CompleteArgs {
    id: i64,
    /// Defaults to true.
    done: Option<bool>,
}

async fn complete(ctx: Ctx, args: CompleteArgs) -> AppResult<()> {
    let at = args.done.unwrap_or(true).then(Utc::now);
    todos::set_done(&ctx.db, args.id, at).await
}

#[derive(Debug, Deserialize, JsonSchema)]
struct IdArgs {
    id: i64,
}

async fn delete(ctx: Ctx, args: IdArgs) -> AppResult<()> {
    todos::delete(&ctx.db, args.id).await
}
