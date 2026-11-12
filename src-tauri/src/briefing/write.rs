//! Saving what the briefing run wrote, after checking it only names this area's projects and
//! references.

use std::collections::HashSet;

use chrono::{DateTime, Utc};

use crate::domain::project::Briefing;
use crate::error::{AppError, AppResult};
use crate::store::{Db, projects, references};

/// A rewritten area: its combined briefing and each project's own.
#[derive(Debug, Clone)]
pub struct Rewrite {
    pub combined: Briefing,
    pub projects: Vec<(String, Briefing)>,
}

fn unknown(what: &str, names: &[String]) -> AppError {
    AppError::BadInput(format!("unknown {what} in this area: {}", names.join(", ")))
}

fn check_projects(briefing: &Briefing, known: &HashSet<String>) -> AppResult<()> {
    let strays: Vec<String> = briefing
        .paragraphs()
        .filter_map(|p| p.project.clone())
        .filter(|p| !known.contains(p))
        .collect();
    if strays.is_empty() {
        Ok(())
    } else {
        Err(unknown("projects", &strays))
    }
}

fn check_citations(briefing: &Briefing, known: &HashSet<i64>) -> AppResult<()> {
    let strays: Vec<String> = briefing
        .cited()
        .into_iter()
        .filter(|id| !known.contains(id))
        .map(|id| id.to_string())
        .collect();
    if strays.is_empty() {
        Ok(())
    } else {
        Err(unknown("references", &strays))
    }
}

/// Checks and saves `area`'s rewrite as of `at`.
pub async fn save(db: &Db, area: &str, rewrite: &Rewrite, at: DateTime<Utc>) -> AppResult<()> {
    let known_projects: HashSet<String> = projects::in_area(db, area)
        .await?
        .into_iter()
        .map(|p| p.id)
        .collect();
    let known_refs: HashSet<i64> = references::in_area(db, area)
        .await?
        .into_iter()
        .map(|r| r.id)
        .collect();
    let strays: Vec<String> = rewrite
        .projects
        .iter()
        .map(|(id, _)| id.clone())
        .filter(|id| !known_projects.contains(id))
        .collect();
    if !strays.is_empty() {
        return Err(unknown("projects", &strays));
    }
    for briefing in std::iter::once(&rewrite.combined).chain(rewrite.projects.iter().map(|p| &p.1))
    {
        check_projects(briefing, &known_projects)?;
        check_citations(briefing, &known_refs)?;
    }
    projects::save_area_briefing(db, area, &rewrite.combined, at).await?;
    for (id, briefing) in &rewrite.projects {
        projects::set_briefing(db, id, briefing, at).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::project::Paragraph;

    fn rewrite(text: &str, project: Option<&str>) -> Rewrite {
        Rewrite {
            combined: Briefing {
                now: vec![Paragraph {
                    project: project.map(str::to_owned),
                    text: text.to_owned(),
                }],
                ..Briefing::default()
            },
            projects: Vec::new(),
        }
    }

    #[tokio::test]
    async fn refuses_citations_and_projects_from_elsewhere() {
        let db = Db::in_memory().await;
        let now = Utc::now();
        assert!(
            save(&db, "work", &rewrite("Stalled[ref:9].", None), now)
                .await
                .is_err()
        );
        assert!(
            save(&db, "work", &rewrite("Stalled.", Some("nope")), now)
                .await
                .is_err()
        );
        save(&db, "work", &rewrite("Stalled.", None), now)
            .await
            .unwrap();
        let stored = projects::area_briefing(&db, "work").await.unwrap().unwrap();
        assert_eq!(stored.briefing.now.len(), 1);
    }
}
