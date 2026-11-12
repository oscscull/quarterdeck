use chrono::Utc;
use schemars::JsonSchema;
use serde::Deserialize;

use super::super::ctx::Ctx;
use super::super::tool::{Tool, async_tool};
use super::in_area;
use crate::config::schedules::slug;
use crate::domain::item::WorkItem;
use crate::domain::project::{Project, ProjectState};
use crate::error::{AppError, AppResult};
use crate::store::{items, projects, references};

const LIST: &str = "Projects: work towards one aim, grouping items within an area (e.g. \"Writer\" at \
work). Highest ranked first. Not repo checkouts: those are list_repos.";
const GET: &str = "One project with its items, its references and its briefing.";
const CREATE: &str = "Starts a project: a title, an area, and its aim in one plain sentence (what the work \
is for, e.g. \"Get the Approvals prototype running on AWS so doctors can submit documents\"). Put items \
in it with update_item's `project`. Returns its id.";
const UPDATE: &str = "Changes a project. Only the fields you pass change. `state`: moving, stalled \
(something outside holds it up), quiet (set aside on purpose) or done. `position` 1 is the top; the top \
projects' aims head the Briefing.";
const DELETE: &str = "Deletes a project and its references. Its items stay, in no project.";

pub fn tools() -> Vec<Tool> {
    vec![
        async_tool("list_projects", LIST, list),
        async_tool("get_project", GET, get),
        async_tool("create_project", CREATE, create),
        async_tool("update_project", UPDATE, update),
        async_tool("delete_project", DELETE, delete),
    ]
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListArgs {
    /// Only this area, by id.
    area: Option<String>,
}

async fn list(ctx: Ctx, args: ListArgs) -> AppResult<Vec<Project>> {
    let mut all = projects::list(&ctx.db).await?;
    all.retain(|p| in_area(args.area.as_ref(), &p.area));
    Ok(all)
}

#[derive(Debug, Deserialize, JsonSchema)]
struct IdArgs {
    id: String,
}

async fn get(ctx: Ctx, args: IdArgs) -> AppResult<serde_json::Value> {
    let project = projects::get(&ctx.db, &args.id).await?;
    let membership = items::projects(&ctx.db).await?;
    let members: Vec<WorkItem> = items::list(&ctx.db)
        .await?
        .into_iter()
        .filter(|i| membership.get(&i.id) == Some(&project.id))
        .collect();
    let sources: Vec<_> = references::in_area(&ctx.db, &project.area)
        .await?
        .into_iter()
        .filter(|r| {
            r.project.as_ref() == Some(&project.id)
                || r.items.iter().any(|i| members.iter().any(|m| &m.id == i))
        })
        .collect();
    Ok(serde_json::json!({ "project": project, "items": members, "references": sources }))
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateArgs {
    title: String,
    /// Area id, e.g. "work".
    area: String,
    /// What the work is for, in one plain sentence.
    aim: String,
    /// A short id, e.g. "writer". Made from the title if left out.
    id: Option<String>,
}

/// `wanted`, or `wanted-2`, `wanted-3`... whichever is free.
async fn free_id(ctx: &Ctx, wanted: &str) -> AppResult<String> {
    let mut id = wanted.to_owned();
    let mut n = 1;
    while projects::find(&ctx.db, &id).await?.is_some() {
        n += 1;
        id = format!("{wanted}-{n}");
    }
    Ok(id)
}

async fn create(ctx: Ctx, args: CreateArgs) -> AppResult<String> {
    let wanted = slug(args.id.as_deref().unwrap_or(&args.title));
    if wanted.is_empty() {
        return Err(AppError::BadInput("a project needs a title".to_owned()));
    }
    let now = Utc::now();
    let project = Project {
        id: free_id(&ctx, &wanted).await?,
        title: args.title,
        aim: args.aim,
        area: ctx.area(&args.area)?,
        state: ProjectState::Moving,
        rank: projects::next_rank(&ctx.db).await?,
        opened_at: now,
        state_since: now,
        briefing: None,
        briefed_at: None,
    };
    projects::save(&ctx.db, &project).await?;
    Ok(project.id)
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateArgs {
    id: String,
    title: Option<String>,
    aim: Option<String>,
    area: Option<String>,
    state: Option<ProjectState>,
    /// Where it goes in the ranking, 1 being the top.
    position: Option<usize>,
}

async fn update(ctx: Ctx, args: UpdateArgs) -> AppResult<()> {
    let mut project = projects::get(&ctx.db, &args.id).await?;
    if let Some(state) = args.state.filter(|s| *s != project.state) {
        project.state = state;
        project.state_since = Utc::now();
    }
    if let Some(area) = args.area {
        project.area = ctx.area(&area)?;
    }
    project.title = args.title.unwrap_or(project.title);
    project.aim = args.aim.unwrap_or(project.aim);
    projects::save(&ctx.db, &project).await?;
    if let Some(position) = args.position {
        let mut ids: Vec<String> = projects::list(&ctx.db)
            .await?
            .into_iter()
            .map(|p| p.id)
            .filter(|id| *id != project.id)
            .collect();
        ids.insert(position.saturating_sub(1).min(ids.len()), project.id);
        projects::reorder(&ctx.db, &ids).await?;
    }
    Ok(())
}

async fn delete(ctx: Ctx, args: IdArgs) -> AppResult<()> {
    projects::delete(&ctx.db, &args.id).await
}

/// Puts an item in a project; an empty `project` takes it out. Shared by `create_item` and `update_item`.
pub async fn assign(ctx: &Ctx, item_id: &str, project: &str) -> AppResult<()> {
    if project.is_empty() {
        return items::set_project(&ctx.db, item_id, None).await;
    }
    let found = projects::get(&ctx.db, project).await?;
    let item = items::get(&ctx.db, item_id).await?;
    if found.area != item.area {
        return Err(AppError::BadInput(format!(
            "{item_id} is in area {} but project {project} is in {}",
            item.area, found.area
        )));
    }
    items::set_project(&ctx.db, item_id, Some(project)).await
}
