//! Why an item has no workspace on this machine.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Missing {
    NoLink,
    /// A link, but not to a GitHub repo, issue or PR.
    NotRepo,
    /// `owner/repo` has no `[[repos]]` checkout here.
    NoProject(String),
}

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoLink => write!(f, "the item has no link"),
            Self::NotRepo => write!(f, "the item's link isn't a GitHub repo, issue or PR"),
            Self::NoProject(repo) => write!(f, "there's no checkout of {repo} on this machine"),
        }
    }
}
