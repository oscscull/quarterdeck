//! Keeps the Briefing's sources from growing forever: references nothing open uses, and projects
//! long finished.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};

use crate::config::settings::Settings;
use crate::domain::item::Stage;
use crate::domain::project::{Project, ProjectState};
use crate::domain::reference::Reference;
use crate::error::AppResult;
use crate::store::{Db, items, projects, references};

/// Whether a reference still supports open work: an open item, or a project not yet done.
fn in_use(
    r: &Reference,
    item_stage: &HashMap<String, Stage>,
    project_state: &HashMap<String, ProjectState>,
) -> bool {
    let open_item = r
        .items
        .iter()
        .any(|i| item_stage.get(i).is_some_and(|s| *s != Stage::Done));
    let live_project = r
        .project
        .as_ref()
        .and_then(|p| project_state.get(p))
        .is_some_and(|s| *s != ProjectState::Done);
    open_item || live_project
}

fn finished_long_ago(p: &Project, before: DateTime<Utc>) -> bool {
    p.state == ProjectState::Done && p.state_since < before
}

/// Deletes references nothing open has used for `reference_ttl_days`, and projects done for
/// `done_item_ttl_days` (their references go with them; their items stay, ungrouped).
pub async fn prune(db: &Db, settings: &Settings, now: DateTime<Utc>) -> AppResult<()> {
    let all_projects = projects::list(db).await?;
    let project_ttl = now - Duration::days(settings.done_item_ttl_days);
    for p in all_projects
        .iter()
        .filter(|p| finished_long_ago(p, project_ttl))
    {
        projects::delete(db, &p.id).await?;
    }
    let project_state: HashMap<String, ProjectState> =
        all_projects.into_iter().map(|p| (p.id, p.state)).collect();
    let item_stage: HashMap<String, Stage> = items::list(db)
        .await?
        .into_iter()
        .map(|i| (i.id.clone(), i.stage()))
        .collect();
    let reference_ttl = now - Duration::days(settings.briefing.reference_ttl_days);
    for area in &settings.areas {
        for r in references::in_area(db, &area.id).await? {
            if r.at < reference_ttl && !in_use(&r, &item_stage, &project_state) {
                references::remove(db, r.id).await?;
            }
        }
    }
    Ok(())
}
