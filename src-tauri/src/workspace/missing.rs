//! Why an item has no workspace on this machine, with the fix the user can make.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Missing {
    NoLink,
    /// A link, but not to a GitHub repo, issue or PR.
    NotRepo,
    /// `owner/repo` has no `[[projects]]` checkout here.
    NoProject(String),
}

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoLink => write!(
                f,
                "the item has no link, so there's no repo to work in (link it to a GitHub repo or issue)"
            ),
            Self::NotRepo => write!(
                f,
                "the item's link isn't a GitHub repo or issue, so there's no repo to work in (link it to one)"
            ),
            Self::NoProject(repo) => write!(
                f,
                "there's no checkout of {repo} on this machine (tell Quarterdeck where it is with set_project)"
            ),
        }
    }
}
