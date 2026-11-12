//! The `[github.issues]` section of `quarterdeck.toml`: whose open issues become items.

use serde::Deserialize;

use super::github::listed;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct IssueSettings {
    /// Owners (`me`) or repos (`my-org/app`) whose open issues become items. Empty watches none.
    pub watch: Vec<String>,
    /// Issues with any of these labels are skipped.
    pub skip_labels: Vec<String>,
}

impl IssueSettings {
    pub fn watches(&self, repo: &str) -> bool {
        listed(&self.watch, repo)
    }

    pub fn skips(&self, labels: &[String]) -> bool {
        labels
            .iter()
            .any(|l| self.skip_labels.iter().any(|s| s.eq_ignore_ascii_case(l)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watches_owners_and_repos_and_skips_labels() {
        let s: IssueSettings =
            toml::from_str("watch = [\"me\", \"acme/app\"]\nskip_labels = [\"wontfix\"]").unwrap();
        assert!(s.watches("me/toy") && s.watches("acme/app"));
        assert!(!s.watches("acme/other"));
        assert!(s.skips(&["WontFix".to_owned()]));
        assert!(!s.skips(&["bug".to_owned()]));
    }
}
