//! The Briefing tab's data for this machine: what the UI shows and peers fetch with `get_briefing`.

use chrono::{Duration, Utc};
use serde::Serialize;

use crate::config::settings::Settings;
use crate::domain::item::{Stage, WorkItem};
use crate::domain::project::{AreaBriefing, Project};
use crate::domain::reference::Reference;
use crate::error::AppResult;
use crate::store::{Db, items, projects, references};

/// A project with its items: open ones, and those finished inside the "just done" window.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    #[serde(flatten)]
    pub project: Project,
    pub items: Vec<WorkItem>,
}

/// Every briefing this machine holds, with what it cites.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BriefingView {
    pub areas: Vec<AreaBriefing>,
    pub projects: Vec<ProjectView>,
    pub references: Vec<Reference>,
    /// Areas whose briefing is being rewritten right now.
    pub writing: Vec<String>,
}

/// The view for `area`, or for every configured area.
pub async fn build(db: &Db, settings: &Settings, area: Option<&str>) -> AppResult<BriefingView> {
    let since = Utc::now() - Duration::days(settings.briefing.just_done_days);
    let wanted: Vec<&str> = settings
        .areas
        .iter()
        .map(|a| a.id.as_str())
        .filter(|a| area.is_none_or(|w| w == *a))
        .collect();
    let membership = items::projects(db).await?;
    let all_items = items::list(db).await?;
    let mut view = BriefingView {
        areas: Vec::new(),
        projects: Vec::new(),
        references: Vec::new(),
        writing: Vec::new(),
    };
    for a in wanted {
        if let Some(b) = projects::area_briefing(db, a).await? {
            view.areas.push(b);
        }
        if super::run::running(db, a).await? {
            view.writing.push(a.to_owned());
        }
        view.references.extend(references::in_area(db, a).await?);
        for project in projects::in_area(db, a).await? {
            let items = all_items
                .iter()
                .filter(|i| membership.get(&i.id) == Some(&project.id))
                .filter(|i| i.stage() != Stage::Done || i.state_since >= since)
                .cloned()
                .collect();
            view.projects.push(ProjectView { project, items });
        }
    }
    Ok(view)
}
