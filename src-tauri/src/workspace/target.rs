//! What an item's work is on GitHub: a PR (from a GitHub item's id), an issue, or new work in a repo
//! (from its link).

use super::Missing;
use crate::connectors::github;
use crate::domain::item::WorkItem;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// An open PR: the worktree sits at its head.
    Pr(u64),
    /// An issue to fix: the worktree is a new branch off the default branch.
    Issue(u64),
    /// New work with no issue, e.g. from a Slack thread: a new branch named after the item, off the
    /// default branch.
    New,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// `owner/repo`.
    pub repo: String,
    pub kind: Kind,
}

/// The PR a GitHub item stands for, from its external id `owner/repo#12`.
fn pr_of(item: &WorkItem) -> Option<Target> {
    if item.source != github::SOURCE {
        return None;
    }
    let (repo, number) = item.external_id.as_deref()?.split_once('#')?;
    Some(Target {
        repo: repo.to_owned(),
        kind: Kind::Pr(number.parse().ok()?),
    })
}

/// `https://github.com/<owner>/<repo>/issues/<n>`.
fn issue_of(link: &str) -> Option<Target> {
    let (repo, number) = github::link::parse(link, "issues")?;
    Some(Target {
        repo,
        kind: Kind::Issue(number),
    })
}

/// `https://github.com/<owner>/<repo>`.
fn repo_of(link: &str) -> Option<Target> {
    Some(Target {
        repo: github::link::repo(link)?,
        kind: Kind::New,
    })
}

/// A PR item's PR, else the issue or repo the item links to.
pub fn of(item: &WorkItem) -> Result<Target, Missing> {
    if let Some(pr) = pr_of(item) {
        return Ok(pr);
    }
    let link = item
        .link
        .as_deref()
        .filter(|l| !l.trim().is_empty())
        .ok_or(Missing::NoLink)?;
    issue_of(link)
        .or_else(|| repo_of(link))
        .ok_or(Missing::NotRepo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn item(source: &str, external_id: Option<&str>, link: Option<&str>) -> WorkItem {
        WorkItem {
            id: "x#1".to_owned(),
            title: "t".to_owned(),
            state_label: String::new(),
            life: crate::domain::life::Life::Unstarted,
            status: None,
            area: "work".to_owned(),
            link: link.map(str::to_owned),
            source: source.to_owned(),
            external_id: external_id.map(str::to_owned),
            opened_at: Utc::now(),
            state_since: Utc::now(),
            rank: 0,
        }
    }

    fn target(repo: &str, kind: Kind) -> Target {
        Target {
            repo: repo.to_owned(),
            kind,
        }
    }

    #[test]
    fn github_items_are_their_pr() {
        let pr = item(
            "github",
            Some("me/app#7"),
            Some("https://github.com/me/app/pull/7"),
        );
        assert_eq!(of(&pr), Ok(target("me/app", Kind::Pr(7))));
    }

    #[test]
    fn items_linking_an_issue_are_that_issue() {
        for link in [
            "https://github.com/oscscull/baseddsl/issues/178",
            "https://github.com/oscscull/baseddsl/issues/178/",
            "https://github.com/oscscull/baseddsl/issues/178#issuecomment-1",
            " https://github.com/oscscull/baseddsl/issues/178?x=1 ",
        ] {
            let issue = item("manual", None, Some(link));
            assert_eq!(
                of(&issue),
                Ok(target("oscscull/baseddsl", Kind::Issue(178))),
                "{link}"
            );
        }
    }

    #[test]
    fn items_linking_a_repo_are_new_work_there() {
        for link in [
            "https://github.com/pol-inc/labbase-compass-webapp",
            "https://github.com/pol-inc/labbase-compass-webapp/",
            "https://github.com/pol-inc/labbase-compass-webapp.git",
            " https://github.com/pol-inc/labbase-compass-webapp?tab=readme ",
        ] {
            let new = item("manual", None, Some(link));
            assert_eq!(
                of(&new),
                Ok(target("pol-inc/labbase-compass-webapp", Kind::New)),
                "{link}"
            );
        }
    }

    #[test]
    fn other_links_say_why_they_have_no_target() {
        for link in [
            "https://github.com/me/app/pull/7",
            "https://github.com/me/app/issues/",
            "https://github.com/me",
            "https://example.com/me/app/issues/3",
            "https://linear.app/x/issue/AB-1",
        ] {
            let other = item("manual", None, Some(link));
            assert_eq!(of(&other), Err(Missing::NotRepo), "{link}");
        }
        assert_eq!(of(&item("manual", None, None)), Err(Missing::NoLink));
        assert_eq!(of(&item("manual", None, Some(" "))), Err(Missing::NoLink));
    }
}
