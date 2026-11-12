//! Events: things that happened, which start the schedules whose triggers match them.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::Serialize;

use super::trigger::OnEvent;

/// The event a trigger watches a path for, given as `when.path`.
pub const FILE_CHANGED: &str = "file.changed";

/// Events agents and scripts make up start with this.
pub const CUSTOM: &str = "custom.";

/// Past this many events in a chain (a run's event starting a run whose event starts…), schedules
/// stop starting each other.
pub const MAX_DEPTH: u32 = 3;

/// One thing that happened.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fired {
    pub id: i64,
    pub kind: String,
    pub area: String,
    /// What it's about: an item or schedule id, or empty.
    pub subject: String,
    /// Its fields, for triggers' `when` filters.
    pub data: BTreeMap<String, String>,
    /// One line for the run, e.g. "CI failing on me/app#7 (https://…)".
    pub about: String,
    /// How many events led to this one.
    pub depth: u32,
    pub at: DateTime<Utc>,
    /// Schedules it started.
    pub started: Vec<String>,
}

impl Fired {
    /// A new event, not yet stored.
    pub fn new(kind: &str, area: &str, subject: &str, about: String) -> Self {
        Self {
            id: 0,
            kind: kind.to_owned(),
            area: area.to_owned(),
            subject: subject.to_owned(),
            data: BTreeMap::new(),
            about,
            depth: 0,
            at: Utc::now(),
            started: Vec::new(),
        }
    }

    #[must_use]
    pub fn with(mut self, field: &str, value: &str) -> Self {
        self.data.insert(field.to_owned(), value.to_owned());
        self
    }

    #[must_use]
    pub const fn at_depth(mut self, depth: u32) -> Self {
        self.depth = depth;
        self
    }

    /// A field by name; `area` and `subject` are fields of every event.
    fn field(&self, name: &str) -> Option<&str> {
        match name {
            "area" => Some(&self.area),
            "subject" => Some(&self.subject),
            _ => self.data.get(name).map(String::as_str),
        }
    }

    /// True if `trigger` is for this event and every filter holds.
    pub fn matches(&self, trigger: &OnEvent) -> bool {
        trigger.on == self.kind
            && trigger
                .when
                .iter()
                .all(|(k, v)| self.field(k).is_some_and(|f| glob(v, f)))
    }
}

/// `*` matches any run of characters; everything else must match exactly.
fn glob(pattern: &str, text: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or_default();
    let Some(mut rest) = text.strip_prefix(first) else {
        return false;
    };
    let parts: Vec<&str> = parts.collect();
    let Some((last, middle)) = parts.split_last() else {
        return rest.is_empty();
    };
    for part in middle {
        let Some(after) = rest.find(part).and_then(|i| rest.get(i + part.len()..)) else {
            return false;
        };
        rest = after;
    }
    rest.ends_with(last)
}

/// An event Quarterdeck can fire, for the trigger editor and agents.
#[derive(Debug, Clone, Copy, Serialize, JsonSchema)]
pub struct Kind {
    pub name: &'static str,
    pub about: &'static str,
    /// Fields `when` can filter on, besides `area` and `subject`.
    pub fields: &'static [&'static str],
}

const fn kind(name: &'static str, about: &'static str, fields: &'static [&'static str]) -> Kind {
    Kind {
        name,
        about,
        fields,
    }
}

const PR: &[&str] = &["item", "pr", "url"];

/// Every event Quarterdeck fires itself. Anything starting `custom.` may also be emitted.
pub const KINDS: &[Kind] = &[
    kind(
        "github.ci_failed",
        "CI started failing on one of the user's PRs.",
        PR,
    ),
    kind(
        "github.ci_passed",
        "CI passed on one of the user's PRs.",
        PR,
    ),
    kind(
        "github.pr_new_comments",
        "New review comments on one of the user's PRs.",
        PR,
    ),
    kind(
        "github.pr_behind",
        "One of the user's PRs fell behind the branch it merges into, which must be up to date to merge.",
        PR,
    ),
    kind(
        "github.pr_conflicts",
        "One of the user's PRs started conflicting with the branch it merges into.",
        PR,
    ),
    kind(
        "item.created",
        "A work item was created. Subject: the item.",
        &["item"],
    ),
    kind(
        "item.stage",
        "A work item moved stage (up_next, in_progress, in_review, done).",
        &["item", "stage"],
    ),
    kind(
        "item.turn",
        "A work item's turn passed (to: you, agent, others, later).",
        &["item", "to"],
    ),
    kind(
        "meeting.starting",
        "A meeting starts in [alerts] lead_minutes (when its countdown appears). Subject: the meeting.",
        &["meeting", "title"],
    ),
    kind(
        "meeting.ended",
        "A meeting ended. Subject: the meeting.",
        &["meeting", "title"],
    ),
    kind(
        "file.changed",
        "A file or folder changed (checked every 30s; a folder's direct contents only). The \
         trigger's `when.path` says which, e.g. \"~/Downloads\"; no wildcards.",
        &["path"],
    ),
    kind(
        "run.finished",
        "A schedule's run finished (outcome: ok, failed). Subject: the schedule.",
        &["schedule", "outcome"],
    ),
];

/// True for a kind a trigger can wait for.
pub fn known(name: &str) -> bool {
    KINDS.iter().any(|k| k.name == name)
        || name
            .strip_prefix(CUSTOM)
            .is_some_and(|rest| !rest.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on(name: &str, when: &[(&str, &str)]) -> OnEvent {
        OnEvent {
            on: name.to_owned(),
            when: when
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            cooldown: 0,
        }
    }

    #[test]
    fn globs() {
        assert!(glob("me/*", "me/app#7"));
        assert!(glob("*#7", "me/app#7"));
        assert!(glob("me/*#*", "me/app#7"));
        assert!(glob("agent", "agent"));
        assert!(!glob("agent", "agents"));
        assert!(!glob("me/*", "you/app#7"));
    }

    #[test]
    fn matches_kind_and_every_filter() {
        let e = Fired::new("item.turn", "work", "B-1", String::new()).with("to", "agent");
        assert!(e.matches(&on("item.turn", &[])));
        assert!(e.matches(&on("item.turn", &[("to", "agent"), ("area", "work")])));
        assert!(!e.matches(&on("item.turn", &[("to", "you")])));
        assert!(!e.matches(&on("item.turn", &[("missing", "*")])));
        assert!(!e.matches(&on("item.stage", &[])));
    }

    #[test]
    fn knows_its_own_kinds_and_custom_ones() {
        assert!(known("github.ci_failed"));
        assert!(known("custom.deploy_done"));
        assert!(!known("custom."));
        assert!(!known("nope"));
    }
}
