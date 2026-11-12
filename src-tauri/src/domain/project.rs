use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How a project is going.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectState {
    /// Work is moving.
    #[default]
    Moving,
    /// Something outside it is holding it up.
    Stalled,
    /// Nothing is happening on it, by choice.
    Quiet,
    Done,
}

/// Work towards one aim, grouping items within an area.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub title: String,
    /// What the work is for, in a sentence.
    pub aim: String,
    pub area: String,
    pub state: ProjectState,
    pub rank: i64,
    pub opened_at: DateTime<Utc>,
    pub state_since: DateTime<Utc>,
    pub briefing: Option<Briefing>,
    pub briefed_at: Option<DateTime<Utc>>,
}

/// One paragraph of a briefing. Citations are `[ref:<id>]` markers in `text`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Paragraph {
    /// The project it's about, if any. Always set in an area's combined briefing.
    pub project: Option<String>,
    pub text: String,
}

/// A plain account of the work: what just finished, what's in flight and why, and what's next.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Briefing {
    #[serde(default)]
    pub done: Vec<Paragraph>,
    #[serde(default)]
    pub now: Vec<Paragraph>,
    #[serde(default)]
    pub next: Vec<Paragraph>,
}

impl Briefing {
    pub fn paragraphs(&self) -> impl Iterator<Item = &Paragraph> {
        self.done.iter().chain(&self.now).chain(&self.next)
    }

    /// The reference ids its citations name, in reading order, each once.
    pub fn cited(&self) -> Vec<i64> {
        let mut ids: Vec<i64> = Vec::new();
        for id in self.paragraphs().flat_map(|p| citations(&p.text)) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids
    }
}

/// The ids in `[ref:<id>]` markers, in order.
pub fn citations(text: &str) -> Vec<i64> {
    text.split("[ref:")
        .skip(1)
        .filter_map(|rest| rest.split_once(']'))
        .filter_map(|(id, _)| id.trim().parse().ok())
        .collect()
}

/// One area's combined briefing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AreaBriefing {
    pub area: String,
    pub briefing: Briefing,
    pub written_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_citations_in_order_once_each() {
        let para = |text: &str| Paragraph {
            project: None,
            text: text.to_owned(),
        };
        let briefing = Briefing {
            done: vec![para("Fixed it[ref:4]. Shipped[ref:2][ref:4].")],
            now: vec![para("Stalled[ref: 7] on [ref:x] [ref:9")],
            next: Vec::new(),
        };
        assert_eq!(briefing.cited(), vec![4, 2, 7]);
    }
}
