//! What Quarterdeck derives from a PR: the item fields it owns, and the signals that fire events.

use serde::{Deserialize, Serialize};

use super::fetch::{Base, Pr, Reviewers};
use crate::domain::item::{Stage, Status};
use crate::domain::schedule::EventKind;

/// The item fields GitHub decides. They are only re-applied when they change upstream, so edits
/// by the user or an agent survive until GitHub says something new.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Owned {
    pub title: String,
    pub stage: Stage,
    pub status: Option<Status>,
    pub label: String,
    pub holder: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub owned: Owned,
    pub ci_failing: bool,
    #[serde(default)]
    pub ci_passed: bool,
    pub comments: u64,
    /// Behind or conflicting with the branch it merges into.
    #[serde(default)]
    pub base: Base,
    /// Informational check results, noted on the item when they change.
    #[serde(default)]
    pub notes: Vec<String>,
}

fn failing(pr: &Pr) -> bool {
    matches!(pr.ci.as_deref(), Some("FAILURE" | "ERROR"))
}

/// Not failing, not conflicting, not a draft, and not yet reviewed either way.
fn waiting_on_review(pr: &Pr) -> bool {
    !failing(pr)
        && pr.base != Base::Conflicts
        && !pr.draft
        && !matches!(pr.review.as_deref(), Some("CHANGES_REQUESTED" | "APPROVED"))
}

/// (status, label, holder) for a PR, most pressing first.
fn state(pr: &Pr) -> (Option<Status>, &'static str, &'static str) {
    if failing(pr) {
        return (Some(Status::CiFailed), "CI failing, needs a fix", "Claude");
    }
    if pr.base == Base::Conflicts {
        return (None, "conflicts with base, needs a fix", "Claude");
    }
    if matches!(pr.ci.as_deref(), Some("PENDING" | "EXPECTED")) {
        return (Some(Status::Ci), "CI running", "CI");
    }
    if pr.draft {
        return (None, "draft", "you");
    }
    match pr.review.as_deref() {
        Some("CHANGES_REQUESTED") => (None, "changes requested", "you"),
        Some("APPROVED") => (Some(Status::Approved), "approved, ready to merge", "you"),
        _ => (Some(Status::Review), "waiting on review", "reviewers"),
    }
}

/// Names the reviewers when a PR waits on them, e.g. "madonoharu".
fn holder_name(pr: &Pr, holder: &str) -> String {
    if holder == "reviewers" && !pr.reviewer_names.is_empty() {
        pr.reviewer_names.join(", ")
    } else {
        holder.to_owned()
    }
}

pub fn of(pr: &Pr) -> Snapshot {
    let (status, label, holder) = match (waiting_on_review(pr), pr.reviewers) {
        (true, Reviewers::Yourself) => (None, "ready for your review", "you"),
        (true, Reviewers::NoneRequested) => (None, "no reviewer requested yet", "you"),
        (true | false, _) => state(pr),
    };
    let holder = holder_name(pr, holder);
    Snapshot {
        owned: Owned {
            title: pr.title.clone(),
            stage: if pr.draft {
                Stage::InProgress
            } else {
                Stage::InReview
            },
            status,
            label: label.to_owned(),
            holder,
        },
        ci_failing: failing(pr),
        ci_passed: pr.ci.as_deref() == Some("SUCCESS"),
        comments: pr.comments,
        base: pr.base,
        notes: pr.notes.clone(),
    }
}

/// Events since the last snapshot. Nothing fires the first time a PR is seen.
pub fn events(old: Option<&Snapshot>, new: &Snapshot) -> Vec<EventKind> {
    let Some(old) = old else { return Vec::new() };
    let mut events = Vec::new();
    if new.comments > old.comments {
        events.push(EventKind::PrNewComments);
    }
    if new.ci_failing && !old.ci_failing {
        events.push(EventKind::CiFailed);
    }
    if new.ci_passed && !old.ci_passed {
        events.push(EventKind::CiPassed);
    }
    if new.base != old.base {
        events.extend(base_event(new.base));
    }
    events
}

/// The event for a branch that fell behind or into conflict.
const fn base_event(base: Base) -> Option<EventKind> {
    match base {
        Base::Current => None,
        Base::Behind => Some(EventKind::PrBehind),
        Base::Conflicts => Some(EventKind::PrConflicts),
    }
}

/// Events the first time a PR is seen on an item Quarterdeck already had (one it opened): CI may
/// have finished before the first sync, and whoever is on the item still needs to hear.
pub fn adopted(new: &Snapshot) -> Vec<EventKind> {
    let mut events = Vec::new();
    if new.ci_failing {
        events.push(EventKind::CiFailed);
    }
    if new.ci_passed {
        events.push(EventKind::CiPassed);
    }
    events.extend(base_event(new.base));
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pr(ci: Option<&str>, review: Option<&str>, comments: u64) -> Pr {
        Pr {
            repo: "me/app".to_owned(),
            number: 1,
            title: "Fix".to_owned(),
            url: String::new(),
            draft: false,
            review: review.map(str::to_owned),
            ci: ci.map(str::to_owned),
            comments,
            reviewers: Reviewers::Requested,
            reviewer_names: Vec::new(),
            closes: Vec::new(),
            base: Base::Current,
            checks: Vec::new(),
            notes: Vec::new(),
        }
    }

    #[test]
    fn failing_ci_wins_over_review_state() {
        let s = of(&pr(Some("FAILURE"), Some("APPROVED"), 0));
        assert_eq!(s.owned.status, Some(Status::CiFailed));
    }

    #[test]
    fn solo_reviews_wait_on_you() {
        let solo = Pr {
            reviewers: Reviewers::Yourself,
            ..pr(Some("SUCCESS"), None, 0)
        };
        assert_eq!(of(&solo).owned.label, "ready for your review");
        let unrequested = Pr {
            reviewers: Reviewers::NoneRequested,
            ..pr(Some("SUCCESS"), None, 0)
        };
        assert_eq!(of(&unrequested).owned.label, "no reviewer requested yet");
        assert_eq!(of(&unrequested).owned.holder, "you");
        let failing = Pr {
            reviewers: Reviewers::Yourself,
            ..pr(Some("FAILURE"), None, 0)
        };
        assert_eq!(of(&failing).owned.status, Some(Status::CiFailed));
    }

    #[test]
    fn waiting_on_review_names_the_reviewers() {
        let named = Pr {
            reviewer_names: vec!["madonoharu".to_owned()],
            ..pr(Some("SUCCESS"), Some("REVIEW_REQUIRED"), 0)
        };
        assert_eq!(of(&named).owned.holder, "madonoharu");
        assert_eq!(of(&pr(Some("SUCCESS"), None, 0)).owned.holder, "reviewers");
    }

    #[test]
    fn conflicts_go_to_claude_and_fire_once() {
        let clean = of(&pr(Some("SUCCESS"), None, 0));
        let conflicting = of(&Pr {
            base: Base::Conflicts,
            ..pr(Some("SUCCESS"), None, 0)
        });
        assert_eq!(conflicting.owned.holder, "Claude");
        assert_eq!(events(Some(&clean), &conflicting), [EventKind::PrConflicts]);
        assert_eq!(events(Some(&conflicting), &conflicting), []);
    }

    #[test]
    fn falling_behind_fires_but_keeps_waiting_on_review() {
        let current = of(&pr(Some("SUCCESS"), None, 0));
        let behind = of(&Pr {
            base: Base::Behind,
            ..pr(Some("SUCCESS"), None, 0)
        });
        assert_eq!(behind.owned, current.owned);
        assert_eq!(events(Some(&current), &behind), [EventKind::PrBehind]);
        assert_eq!(events(Some(&behind), &current), []);
    }

    #[test]
    fn events_fire_on_change_only() {
        let before = of(&pr(Some("SUCCESS"), None, 2));
        let after = of(&pr(Some("FAILURE"), None, 3));
        let fixed = of(&pr(Some("SUCCESS"), None, 3));
        assert_eq!(events(Some(&after), &fixed), [EventKind::CiPassed]);
        assert_eq!(events(None, &after), []);
        assert_eq!(events(Some(&before), &before), []);
        assert_eq!(
            events(Some(&before), &after),
            [EventKind::PrNewComments, EventKind::CiFailed]
        );
    }

    #[test]
    fn an_adopted_pr_reports_ci_that_finished_before_the_first_sync() {
        let failed = of(&pr(Some("FAILURE"), None, 3));
        assert_eq!(adopted(&failed), [EventKind::CiFailed]);
        assert_eq!(
            adopted(&of(&pr(Some("SUCCESS"), None, 3))),
            [EventKind::CiPassed]
        );
        assert_eq!(adopted(&of(&pr(Some("PENDING"), None, 3))), []);
    }
}
