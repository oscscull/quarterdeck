//! What needs the user, derived from whose turn each item is and ranked by the work. Never ordered
//! by hand.
//!
//! In rank order: questions raised on an item, else the item itself when it's the user's turn,
//! saying what it needs. `attention` decides which items those are.

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::attention::{self, Attention};
use crate::domain::ask::Ask;
use crate::domain::item::WorkItem;
use crate::domain::life::Whose;

/// Why something is a priority. The UI words each kind from the copy file.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Reason {
    /// A question or design fork someone raised on the item.
    Ask { ask: Ask },
    /// It's the user's turn on the item, e.g. a PR to review or merge.
    Yours { need: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Priority {
    #[serde(flatten)]
    pub reason: Reason,
    pub item_id: String,
    pub item_title: String,
    pub item_label: String,
    pub area: String,
    /// When the item reached its current state.
    pub since: DateTime<Utc>,
    /// The item's ticket or PR, so the user can act on it in one click.
    pub link: Option<String>,
}

fn priority(item: &WorkItem, reason: Reason) -> Priority {
    Priority {
        reason,
        item_id: item.id.clone(),
        item_title: item.title.clone(),
        item_label: item.state_label.clone(),
        area: item.area.clone(),
        since: item.state_since,
        link: item.link.clone(),
    }
}

/// What the user's turn on the item needs, if it's theirs.
fn need(item: &WorkItem) -> Option<String> {
    match item.life.whose()? {
        Whose::You { need } => Some(need.clone()),
        Whose::Agent | Whose::Others { .. } | Whose::Later { .. } => None,
    }
}

/// Questions on the item, or the item itself when it's the user's turn.
fn needs_you(item: &WorkItem, asks: &[Ask]) -> Vec<Priority> {
    let mine: Vec<Priority> = asks
        .iter()
        .filter(|a| a.item_id == item.id)
        .map(|a| priority(item, Reason::Ask { ask: a.clone() }))
        .collect();
    if attention::of(item, !mine.is_empty()) != Attention::Today {
        return Vec::new();
    }
    if mine.is_empty() {
        let need = need(item).unwrap_or_default();
        return vec![priority(item, Reason::Yours { need })];
    }
    mine
}

/// Every priority, in order. `items` must already be sorted by rank.
pub fn priorities(items: &[WorkItem], asks: &[Ask]) -> Vec<Priority> {
    items.iter().flat_map(|i| needs_you(i, asks)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::life::{Life, Phase};

    fn item(id: &str, whose: Option<Whose>) -> WorkItem {
        let life = whose.map_or_else(Life::not_started, |w| {
            Life::open_as(Phase::InReview, w, None)
        });
        WorkItem::sample(id, life)
    }

    fn you(need: &str) -> Whose {
        Whose::You {
            need: need.to_owned(),
        }
    }

    fn ask(id: i64, item_id: &str) -> Ask {
        Ask {
            id,
            item_id: item_id.to_owned(),
            ..Ask::default()
        }
    }

    fn kinds(list: &[Priority]) -> Vec<String> {
        list.iter()
            .map(|p| match &p.reason {
                Reason::Ask { ask } => format!("ask{}", ask.id),
                Reason::Yours { need } => format!("{}: {need}", p.item_id),
            })
            .collect()
    }

    #[test]
    fn the_users_turns_and_every_question_in_rank_order() {
        let others = Whose::Others {
            waiting_on: "Sam".to_owned(),
            then: None,
        };
        let items = [
            item("reviewing", Some(others)),
            item("mine", Some(you("Merge it?"))),
            item("asked", Some(Whose::Agent)),
            item("working", Some(Whose::Agent)),
            item("someday", None),
            item("ready", Some(you("Pick a name"))),
            WorkItem::sample("shipped", Life::Done),
        ];
        let asks = [ask(7, "asked"), ask(8, "shipped")];
        assert_eq!(
            kinds(&priorities(&items, &asks)),
            [
                "mine: Merge it?",
                "ask7",
                "someday: Not started: start it, park it until it can, or close it",
                "ready: Pick a name",
                "ask8"
            ]
        );
    }

    #[test]
    fn a_question_stands_in_for_the_item_itself() {
        let items = [item("mine", Some(you("Merge?")))];
        assert_eq!(kinds(&priorities(&items, &[ask(1, "mine")])), ["ask1"]);
    }
}
