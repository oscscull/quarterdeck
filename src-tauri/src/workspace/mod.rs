//! Work Quarterdeck starts on a PR, or on an item linking a GitHub issue, runs in that repo's project,
//! in a git worktree of its own per item. The user's checkout is never touched, and the project's
//! own Claude settings apply. A worktree lives while its item is open; the project's `clean_after_run`
//! folders are deleted from it after every run.

mod clean;
mod git;
mod issue;
mod publish;
mod sweep;
mod target;

use std::path::PathBuf;

pub use clean::clean;
pub use git::prepare;
pub use issue::text as issue_text;
pub use publish::{Published, publish};
pub use sweep::sweep;
pub use target::Kind;

use crate::config::Config;
use crate::config::projects;
use crate::config::prompts::{Prompt, render};
use crate::config::settings::Settings;
use crate::domain::item::WorkItem;
use crate::domain::session::SessionRef;
use crate::error::AppResult;
use crate::paths;

/// Where one item's work happens.
#[derive(Debug, Clone)]
pub struct Workspace {
    /// The user's checkout of the repo.
    pub project: PathBuf,
    pub repo: String,
    /// The PR's or the issue's number.
    pub number: u64,
    pub kind: Kind,
    /// The item's own worktree.
    pub dir: PathBuf,
    /// The project's `clean_after_run` folders, deleted from `dir` after every run.
    pub clean_after_run: Vec<String>,
}

impl Workspace {
    /// A new session that will run in this worktree.
    pub fn new_session(&self) -> SessionRef {
        SessionRef {
            id: uuid::Uuid::new_v4().to_string(),
            cwd: Some(self.dir.display().to_string()),
        }
    }

    pub fn holds(&self, session: &SessionRef) -> bool {
        session.cwd.as_deref().map(PathBuf::from) == Some(self.dir.clone())
    }

    /// The branch Quarterdeck makes to fix an issue.
    pub fn issue_branch(&self) -> String {
        format!("issue-{}", self.number)
    }
}

fn root() -> AppResult<PathBuf> {
    Ok(paths::data_dir()?.join("worktrees"))
}

/// A folder name for an item id, e.g. `app#12` → `app-12`.
fn folder(item_id: &str) -> String {
    item_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// The workspace for an item, if it's a PR, or links an issue, in a repo with a project on this
/// machine.
pub fn for_item(settings: &Settings, item: &WorkItem) -> AppResult<Option<Workspace>> {
    let Some(t) = target::of(item) else {
        return Ok(None);
    };
    let Some(project) = projects::find(&settings.projects, &t.repo) else {
        return Ok(None);
    };
    Ok(Some(Workspace {
        project: project.dir(),
        repo: t.repo,
        number: t.number,
        kind: t.kind,
        dir: root()?.join(folder(&item.id)),
        clean_after_run: project.clean_after_run.clone(),
    }))
}

/// Tells a run where it is: in a worktree it can push from, or with no checkout at all.
pub fn describe(config: &Config, ws: Option<&Workspace>) -> AppResult<String> {
    ws.map_or_else(
        || render(config, Prompt::NoWorkspace, &[]),
        |ws| {
            let number = ws.number.to_string();
            let branch = ws.issue_branch();
            let vars = [
                ("repo", ws.repo.as_str()),
                ("number", number.as_str()),
                ("branch", branch.as_str()),
            ];
            let prompt = match ws.kind {
                Kind::Pr => Prompt::Workspace,
                Kind::Issue => Prompt::IssueWorkspace,
            };
            render(config, prompt, &vars)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::config::projects::Project;
    use crate::domain::item::Stage;

    #[test]
    fn folders_are_safe_names() {
        assert_eq!(folder("labbase-webapp#1928"), "labbase-webapp-1928");
    }

    fn issue_item(link: &str) -> WorkItem {
        WorkItem {
            id: "baseddsl#178".to_owned(),
            title: "t".to_owned(),
            state_label: String::new(),
            stage: Stage::UpNext,
            status: None,
            turn: crate::domain::turn::Turn::You,
            turn_until: None,
            waiting_on: None,
            then_turn: None,
            area: "projects".to_owned(),
            link: Some(link.to_owned()),
            source: "manual".to_owned(),
            external_id: None,
            opened_at: chrono::Utc::now(),
            state_since: chrono::Utc::now(),
            rank: 0,
        }
    }

    #[test]
    fn issues_in_known_projects_get_a_branch_of_their_own() {
        let settings = Settings {
            projects: vec![Project {
                repo: "OscScull/BasedDSL".to_owned(),
                path: "/code/baseddsl".to_owned(),
                clean_after_run: Vec::new(),
            }],
            ..Settings::default()
        };
        let item = issue_item("https://github.com/oscscull/baseddsl/issues/178");
        let ws = for_item(&settings, &item).unwrap().unwrap();
        assert_eq!(
            (ws.kind, ws.number, ws.issue_branch()),
            (Kind::Issue, 178, "issue-178".to_owned())
        );
        assert_eq!(ws.project, PathBuf::from("/code/baseddsl"));
        assert!(ws.dir.ends_with("worktrees/baseddsl-178"));

        let elsewhere = issue_item("https://github.com/someone/else/issues/1");
        assert!(for_item(&settings, &elsewhere).unwrap().is_none());
    }
}
