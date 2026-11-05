//! Work Quarterdeck starts on a PR, or on an item linking a GitHub issue or repo, runs in that repo's
//! project, in a git worktree of its own per item. The user's checkout is never touched, and the project's
//! own Claude settings apply. A worktree lives while its item is open.

mod git;
mod issue;
mod missing;
mod publish;
mod sweep;
mod target;

use std::path::PathBuf;

pub use git::prepare;
pub use issue::text as issue_text;
pub use missing::Missing;
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
    pub kind: Kind,
    /// The item's own worktree.
    pub dir: PathBuf,
    /// The branch Quarterdeck makes for the work. None for a PR, which has its own.
    pub branch: Option<String>,
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

/// The branch for new work: `issue-12` for an issue, `quarterdeck/<item>` for other new work.
fn branch(kind: Kind, item_id: &str) -> Option<String> {
    match kind {
        Kind::Pr(_) => None,
        Kind::Issue(n) => Some(format!("issue-{n}")),
        Kind::New => Some(format!("quarterdeck/{}", folder(item_id).to_lowercase())),
    }
}

/// An item's workspace on this machine, or why it has none.
pub type Located = Result<Workspace, Missing>;

/// The workspace for an item that's a PR, or links an issue or repo, with a project on this
/// machine; else why there's none.
pub fn locate(settings: &Settings, item: &WorkItem) -> AppResult<Located> {
    let t = match target::of(item) {
        Ok(t) => t,
        Err(missing) => return Ok(Err(missing)),
    };
    let Some(project) = projects::find(&settings.projects, &t.repo) else {
        return Ok(Err(Missing::NoProject(t.repo)));
    };
    Ok(Ok(Workspace {
        project: project.dir(),
        branch: branch(t.kind, &item.id),
        repo: t.repo,
        kind: t.kind,
        dir: root()?.join(folder(&item.id)),
    }))
}

/// Tells a run where it is: in a worktree it can push from, or with no checkout at all and why.
pub fn describe(config: &Config, located: Result<&Workspace, &Missing>) -> AppResult<String> {
    located.map_or_else(
        |missing| {
            render(
                config,
                Prompt::NoWorkspace,
                &[("why", &missing.to_string())],
            )
        },
        |ws| {
            let (prompt, number) = match ws.kind {
                Kind::Pr(n) => (Prompt::Workspace, n.to_string()),
                Kind::Issue(n) => (Prompt::IssueWorkspace, n.to_string()),
                Kind::New => (Prompt::NewWorkspace, String::new()),
            };
            let vars = [
                ("repo", ws.repo.as_str()),
                ("number", number.as_str()),
                ("branch", ws.branch.as_deref().unwrap_or_default()),
            ];
            render(config, prompt, &vars)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::config::projects::Project;

    #[test]
    fn folders_are_safe_names() {
        assert_eq!(folder("labbase-webapp#1928"), "labbase-webapp-1928");
    }

    fn linking(id: &str, link: &str) -> WorkItem {
        WorkItem {
            id: id.to_owned(),
            title: "t".to_owned(),
            state_label: String::new(),
            life: crate::domain::life::Life::not_started(),
            status: None,
            area: "projects".to_owned(),
            link: Some(link.to_owned()),
            source: "manual".to_owned(),
            external_id: None,
            opened_at: chrono::Utc::now(),
            state_since: chrono::Utc::now(),
            rank: 0,
        }
    }

    fn settings() -> Settings {
        Settings {
            projects: vec![Project {
                repo: "OscScull/BasedDSL".to_owned(),
                path: "/code/baseddsl".to_owned(),
            }],
            ..Settings::default()
        }
    }

    #[test]
    fn issues_in_known_projects_get_a_branch_of_their_own() {
        let item = linking(
            "baseddsl#178",
            "https://github.com/oscscull/baseddsl/issues/178",
        );
        let ws = locate(&settings(), &item).unwrap().unwrap();
        assert_eq!(
            (ws.kind, ws.branch.as_deref()),
            (Kind::Issue(178), Some("issue-178"))
        );
        assert_eq!(ws.project, PathBuf::from("/code/baseddsl"));
        assert!(ws.dir.ends_with("worktrees/baseddsl-178"));
    }

    #[test]
    fn repo_links_in_known_projects_get_a_branch_named_after_the_item() {
        let item = linking("B-15", "https://github.com/oscscull/baseddsl");
        let ws = locate(&settings(), &item).unwrap().unwrap();
        assert_eq!(
            (ws.kind, ws.branch.as_deref()),
            (Kind::New, Some("quarterdeck/b-15"))
        );
        assert!(ws.dir.ends_with("worktrees/B-15"));
    }

    #[test]
    fn items_with_no_workspace_say_why() {
        let elsewhere = linking("x", "https://github.com/someone/else/issues/1");
        assert_eq!(
            locate(&settings(), &elsewhere).unwrap().err(),
            Some(Missing::NoProject("someone/else".to_owned()))
        );
        let slack = linking("x", "https://slack.com/archives/C1/p2");
        assert_eq!(
            locate(&settings(), &slack).unwrap().err(),
            Some(Missing::NotRepo)
        );
    }
}
