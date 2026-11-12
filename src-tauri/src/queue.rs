//! Items the agent works on one at a time, in the order `[queue]` lists them. The first one not
//! yet in review or done is current: it starts when it's up next, and the queue waits while it's in
//! progress. Up-next work that is parked or waiting can't start, so it's passed over until it can. Starting one gives the agent the turn and `prompts/queue.md`.

use crate::domain::ask::Ask;
use crate::domain::item::{Stage, WorkItem};
use crate::domain::turn::Turn;
use crate::error::AppResult;
use crate::store::{Db, asks};
use crate::turns::{self, Clock, Pass};

#[derive(Debug, Clone, Copy)]
pub enum Next<'a> {
    /// Up next: start it.
    Start(&'a WorkItem),
    /// Already in progress: wait for it.
    Wait(&'a WorkItem),
    /// Everything listed is in review or done, or gone.
    Finished,
}

pub fn next<'a>(queue: &[String], items: &'a [WorkItem]) -> Next<'a> {
    for id in queue {
        let Some(item) = items.iter().find(|i| &i.id == id) else {
            continue;
        };
        match item.stage() {
            Stage::InReview | Stage::Done => {}
            // Parked or waiting before it starts: it can't start yet, so the queue goes on.
            Stage::UpNext if item.is_turn(Turn::Later) || item.is_turn(Turn::Others) => {}
            Stage::UpNext => return Next::Start(item),
            Stage::InProgress => return Next::Wait(item),
        }
    }
    Next::Finished
}

/// Leaves `message` for the scheduler to take to Claude, and gives the agent the turn, which starts
/// the item, so the queue waits for it.
pub async fn start(db: &Db, item: &WorkItem, message: &str, clock: Clock<'_>) -> AppResult<()> {
    let ask = Ask {
        item_id: item.id.clone(),
        ..Ask::default()
    };
    let id = asks::insert(db, &ask).await?;
    asks::answer(db, id, message).await?;
    let pass = Pass::to(Turn::Agent, "next in the queue");
    turns::pass(db, &item.id, pass, clock).await
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::config::turns::TurnSettings;
    use crate::store::items;

    fn item(id: &str, stage: Stage) -> WorkItem {
        WorkItem {
            area: "projects".to_owned(),
            source: "manual".to_owned(),
            ..WorkItem::sample(id, crate::lifecycle::new(stage, ""))
        }
    }

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[tokio::test]
    async fn works_through_items_one_at_a_time_in_order() {
        let all = [
            item("a", Stage::InReview),
            item("b", Stage::UpNext),
            item("c", Stage::UpNext),
            item("d", Stage::InProgress),
        ];
        assert!(
            matches!(next(&ids(&["a", "gone", "b", "c"]), &all), Next::Start(i) if i.id == "b")
        );
        assert!(matches!(next(&ids(&["d", "b"]), &all), Next::Wait(i) if i.id == "d"));
        assert!(matches!(next(&ids(&["a"]), &all), Next::Finished));
        assert!(matches!(next(&[], &all), Next::Finished));
        let parked = WorkItem {
            life: crate::domain::life::Life::open_as(
                crate::domain::life::Phase::UpNext,
                crate::domain::life::Whose::Later {
                    then: None,
                    wake: Vec::new(),
                    note: Some("needs the new API".to_owned()),
                },
                None,
            ),
            ..item("p", Stage::UpNext)
        };
        let with_parked = [parked, item("c", Stage::UpNext)];
        assert!(matches!(next(&ids(&["p", "c"]), &with_parked), Next::Start(i) if i.id == "c"));
    }

    #[tokio::test]
    async fn starting_leaves_a_message_and_holds_the_queue() {
        let db = Db::in_memory().await;
        let up = item("b", Stage::UpNext);
        items::save(&db, &up).await.unwrap();
        start(&db, &up, "Fix it.", Clock::now(&TurnSettings::default()))
            .await
            .unwrap();
        let waiting = asks::answered(&db).await.unwrap();
        assert_eq!(waiting.first().unwrap().answer, "Fix it.");
        let all = items::list(&db).await.unwrap();
        assert!(matches!(next(&ids(&["b"]), &all), Next::Wait(_)));
    }
}
