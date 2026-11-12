use chrono::Utc;
use schemars::JsonSchema;
use serde::Deserialize;

use super::super::ctx::Ctx;
use super::super::tool::{Tool, async_tool};
use crate::domain::reference::{RefBy, RefKind, Reference};
use crate::error::{AppError, AppResult};
use crate::store::references::{self, Source};
use crate::store::{items, projects};

const ADD: &str = "Adds a reference: a source the Briefing can cite, such as something the user said, a \
ticket, a PR or its comment, a Slack thread, a doc or a commit. `quote` is the data point that matters, \
quoted or in a plain line. Link it to the `items` and `project` it's about. Returns its id, cited as \
`[ref:<id>]`. References nothing open uses are deleted after a while.";
const UPDATE: &str =
    "Rewrites a reference. Only the fields you pass change; `items` replaces the list.";
const LIST: &str =
    "References in an area, newest first, optionally only those about one project or item.";

pub fn tools() -> Vec<Tool> {
    vec![
        async_tool("add_reference", ADD, add),
        async_tool("update_reference", UPDATE, update),
        async_tool(
            "remove_reference",
            "Deletes a reference that's wrong or no longer matters.",
            remove,
        ),
        async_tool("list_references", LIST, list),
    ]
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AddArgs {
    /// Area id, e.g. "work". Taken from the project or first item when left out.
    area: Option<String>,
    kind: RefKind,
    /// A few words naming it, e.g. "Approvals blocked on AWS".
    title: String,
    url: Option<String>,
    /// The data point that matters, quoted or in a plain line.
    quote: String,
    /// `you` when it's the user's own words or something they gave; `agent` otherwise.
    by: RefBy,
    project: Option<String>,
    /// Ids of the items it's about.
    #[serde(default)]
    items: Vec<String>,
}

/// The area a reference belongs in: given, else its project's, else its first item's.
async fn area_of(ctx: &Ctx, args: &AddArgs) -> AppResult<String> {
    if let Some(area) = &args.area {
        return ctx.area(area);
    }
    if let Some(project) = &args.project {
        return Ok(projects::get(&ctx.db, project).await?.area);
    }
    match args.items.first() {
        Some(item) => Ok(items::get(&ctx.db, item).await?.area),
        None => Err(AppError::BadInput(
            "say which area, project or items it's about".to_owned(),
        )),
    }
}

async fn check_links(ctx: &Ctx, project: Option<&String>, linked: &[String]) -> AppResult<()> {
    if let Some(p) = project {
        projects::get(&ctx.db, p).await?;
    }
    for item in linked {
        items::get(&ctx.db, item).await?;
    }
    Ok(())
}

async fn add(ctx: Ctx, args: AddArgs) -> AppResult<i64> {
    check_links(&ctx, args.project.as_ref(), &args.items).await?;
    let area = area_of(&ctx, &args).await?;
    let source = Source {
        kind: args.kind,
        title: args.title,
        url: args.url,
        quote: args.quote,
        project: args.project,
        items: args.items,
    };
    references::add(&ctx.db, (&area, args.by, Utc::now()), &source).await
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateArgs {
    id: i64,
    kind: Option<RefKind>,
    title: Option<String>,
    url: Option<String>,
    quote: Option<String>,
    /// A project id; "" takes it out of its project.
    project: Option<String>,
    items: Option<Vec<String>>,
}

async fn update(ctx: Ctx, args: UpdateArgs) -> AppResult<()> {
    let old = references::get(&ctx.db, args.id).await?;
    let project = match args.project {
        Some(p) if p.is_empty() => None,
        Some(p) => Some(p),
        None => old.project,
    };
    let linked = args.items.unwrap_or(old.items);
    check_links(&ctx, project.as_ref(), &linked).await?;
    let source = Source {
        kind: args.kind.unwrap_or(old.kind),
        title: args.title.unwrap_or(old.title),
        url: args.url.or(old.url),
        quote: args.quote.unwrap_or(old.quote),
        project,
        items: linked,
    };
    references::update(&ctx.db, args.id, &source).await
}

#[derive(Debug, Deserialize, JsonSchema)]
struct IdArgs {
    id: i64,
}

async fn remove(ctx: Ctx, args: IdArgs) -> AppResult<()> {
    references::remove(&ctx.db, args.id).await
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListArgs {
    area: String,
    project: Option<String>,
    item: Option<String>,
}

async fn list(ctx: Ctx, args: ListArgs) -> AppResult<Vec<Reference>> {
    let mut all = references::in_area(&ctx.db, &ctx.area(&args.area)?).await?;
    all.retain(|r| {
        args.project
            .as_ref()
            .is_none_or(|p| r.project.as_ref() == Some(p))
            && args.item.as_ref().is_none_or(|i| r.items.contains(i))
    });
    Ok(all)
}
