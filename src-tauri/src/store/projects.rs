use chrono::{DateTime, Utc};

use super::client::{self, AreaBriefingRow, ProjectRow};
use super::convert::{same, text_key};
use super::db::{Db, gone_ok};
use super::time;
use crate::domain::project::{AreaBriefing, Briefing, Project};
use crate::error::{AppError, AppResult};

/// A stored briefing that no longer reads is treated as unwritten: the next run replaces it.
fn briefing(stored: Option<serde_json::Value>) -> Option<Briefing> {
    stored.and_then(|b| serde_json::from_value(b).ok())
}

fn from_row(row: ProjectRow) -> AppResult<Project> {
    Ok(Project {
        id: row.id.into_raw(),
        title: row.title,
        aim: row.aim,
        area: row.area,
        state: same(&row.state)?,
        rank: row.rank,
        opened_at: time::get(&row.opened_at)?,
        state_since: time::get(&row.state_since)?,
        briefing: briefing(row.briefing),
        briefed_at: time::get_opt(row.briefed_at.as_deref())?,
    })
}

/// Every project, highest ranked first.
pub async fn list(db: &Db) -> AppResult<Vec<Project>> {
    let rows = db.api().projects(client::ProjectsInput {}, ()).await?;
    rows.into_iter().map(from_row).collect()
}

/// One area's projects, highest ranked first.
pub async fn in_area(db: &Db, area: &str) -> AppResult<Vec<Project>> {
    let input = client::ProjectsInInput {
        area: area.to_owned(),
    };
    let rows = db.api().projects_in(input, ()).await?;
    rows.into_iter().map(from_row).collect()
}

pub async fn find(db: &Db, id: &str) -> AppResult<Option<Project>> {
    let input = client::ProjectInput { id: text_key(id) };
    db.api().project(input, ()).await?.map(from_row).transpose()
}

pub async fn get(db: &Db, id: &str) -> AppResult<Project> {
    find(db, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("project {id}")))
}

/// The rank that puts a new project last.
pub async fn next_rank(db: &Db) -> AppResult<i64> {
    let row = db
        .api()
        .top_project_rank(client::TopProjectRankInput {}, ())
        .await?;
    Ok(row.and_then(|r| r.top).map_or(0, |top| top + 1))
}

/// Inserts or replaces a project by id. Its briefing is left alone.
pub async fn save(db: &Db, p: &Project) -> AppResult<()> {
    let input = client::SaveProjectInput {
        id: text_key(&p.id),
        title: p.title.clone(),
        aim: p.aim.clone(),
        area: p.area.clone(),
        state: same(&p.state)?,
        rank: p.rank,
        opened_at: time::put(p.opened_at),
        state_since: time::put(p.state_since),
    };
    Ok(db.api().save_project(input, ()).await?)
}

pub async fn set_briefing(
    db: &Db,
    id: &str,
    briefing: &Briefing,
    at: DateTime<Utc>,
) -> AppResult<()> {
    let input = client::SetProjectBriefingInput {
        id: text_key(id),
        briefing: serde_json::to_value(briefing)?,
        briefed_at: time::put(at),
    };
    Ok(db.api().set_project_briefing(input, ()).await?)
}

/// Deletes a project and its references. Its items stay, ungrouped.
pub async fn delete(db: &Db, id: &str) -> AppResult<()> {
    let input = client::DeleteProjectInput { id: text_key(id) };
    gone_ok(
        db.api()
            .delete_project(input, ())
            .await
            .map_err(AppError::from),
    )
}

/// Sets each project's rank to its position in `ids`.
pub async fn reorder(db: &Db, ids: &[String]) -> AppResult<()> {
    for (rank, id) in (0_i64..).zip(ids) {
        let mut project = get(db, id).await?;
        project.rank = rank;
        save(db, &project).await?;
    }
    Ok(())
}

fn area_from_row(row: AreaBriefingRow) -> AppResult<AreaBriefing> {
    Ok(AreaBriefing {
        area: row.area.into_raw(),
        briefing: serde_json::from_value(row.text).unwrap_or_default(),
        written_at: time::get(&row.written_at)?,
    })
}

/// An area's combined briefing, if one has been written.
pub async fn area_briefing(db: &Db, area: &str) -> AppResult<Option<AreaBriefing>> {
    let input = client::AreaBriefingInput {
        area: text_key(area),
    };
    db.api()
        .area_briefing(input, ())
        .await?
        .map(area_from_row)
        .transpose()
}

pub async fn save_area_briefing(
    db: &Db,
    area: &str,
    briefing: &Briefing,
    at: DateTime<Utc>,
) -> AppResult<()> {
    let input = client::SaveAreaBriefingInput {
        area: text_key(area),
        text: serde_json::to_value(briefing)?,
        written_at: time::put(at),
    };
    Ok(db.api().save_area_briefing(input, ()).await?)
}
