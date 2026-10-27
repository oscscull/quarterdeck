//! An answer or message nobody can take: no agent session holds the item and there's no worktree to
//! start one in. It isn't dropped quietly: the turn comes back to the user with why, and they get an
//! urgent item with what they said, replacing any earlier one for the item so retries don't pile up.

use crate::domain::ask::Answered;
use crate::domain::turn::Turn;
use crate::domain::urgent::Urgent;
use crate::error::AppResult;
use crate::store::{Db, items, urgent};
use crate::turns::{self, Clock, Pass};

/// `missing` says why there's no worktree, and how to fix it.
pub async fn report(
    db: &Db,
    answered: &Answered,
    missing: &str,
    clock: Clock<'_>,
) -> AppResult<()> {
    let now = clock.now;
    let item = items::get(db, &answered.ask.item_id).await?;
    let reason = format!("no agent session holds it, and {missing}");
    let why = format!("not passed to the agent: {reason}");
    turns::pass(db, &item.id, Pass::to(Turn::You, why), clock).await?;
    let title = format!("Your agent couldn't take your message on {}", item.id);
    replace_earlier(db, &title).await?;
    let raised = Urgent {
        id: 0,
        title,
        detail: format!("“{}”: {reason}.", answered.answer),
        area: item.area.clone(),
        raised_at: now,
        schedule_id: None,
        run_id: None,
    };
    urgent::raise(db, &raised).await?;
    Ok(())
}

/// Dismisses the open urgent items with this title.
async fn replace_earlier(db: &Db, title: &str) -> AppResult<()> {
    for earlier in urgent::list(db).await? {
        if earlier.title == title {
            urgent::dismiss(db, earlier.id).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::config::turns::TurnSettings;
    use crate::domain::ask::Ask;
    use crate::domain::event::Change;
    use crate::domain::item::{ItemInput, Stage};
    use crate::store::events;
    use crate::work;

    /// An item whose turn is the agent's after the user said "fix it".
    async fn told(db: &Db, limits: &TurnSettings) -> Answered {
        let input = ItemInput {
            id: "B-1".to_owned(),
            title: "t".to_owned(),
            state_label: "acting on your answer".to_owned(),
            stage: Stage::InProgress,
            status: None,
            area: "work".to_owned(),
            link: None,
        };
        work::save(db, input, Utc::now()).await.unwrap();
        turns::pass(
            db,
            "B-1",
            Pass::to(Turn::Agent, "you said: fix it"),
            Clock::now(limits),
        )
        .await
        .unwrap();
        Answered {
            ask: Ask {
                item_id: "B-1".to_owned(),
                ..Ask::default()
            },
            answer: "fix it".to_owned(),
        }
    }

    #[tokio::test]
    async fn undeliverable_messages_reach_the_user_instead() {
        let db = Db::in_memory().await;
        let limits = TurnSettings::default();
        let answered = told(&db, &limits).await;
        let missing = "the item has no link";
        report(&db, &answered, missing, Clock::now(&limits))
            .await
            .unwrap();
        report(&db, &answered, missing, Clock::now(&limits))
            .await
            .unwrap();

        assert!(items::get(&db, "B-1").await.unwrap().is_turn(Turn::You));
        let raised = urgent::list(&db).await.unwrap();
        assert_eq!(raised.len(), 1, "a retry replaces the earlier urgent item");
        let detail = &raised.first().unwrap().detail;
        assert!(detail.contains("fix it") && detail.contains(missing));
        let history = events::for_item(&db, "B-1").await.unwrap();
        assert!(history.iter().any(|e| matches!(
            &e.change,
            Change::Turn { why, .. } if why.starts_with("not passed to the agent")
        )));
    }
}
