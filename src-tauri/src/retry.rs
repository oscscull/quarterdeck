//! Retrying a failed agent run from its urgent card: the item goes back to the agent, and its next
//! prompt is the failed run's own, asked again.

use crate::config::turns::TurnSettings;
use crate::domain::turn::Turn;
use crate::error::{AppError, AppResult};
use crate::store::run_prompts::{self, Asked};
use crate::store::{Db, meta, urgent};
use crate::turns::{self, Clock, Pass};

fn pending(item_id: &str) -> String {
    format!("retry.{item_id}")
}

/// Gives the card's item back to the agent to ask its failed run again, and clears the card.
pub async fn request(db: &Db, urgent_id: i64, limits: &TurnSettings) -> AppResult<()> {
    let card = urgent::find(db, urgent_id).await?;
    let run = card
        .run_id
        .ok_or_else(|| AppError::BadInput("this alert has no run to retry".to_owned()))?;
    let asked = run_prompts::get(db, run).await?;
    queue(db, &asked.item_id, run).await?;
    let why = format!("retrying {} after it failed", asked.what);
    turns::pass(
        db,
        &asked.item_id,
        Pass::to(Turn::Agent, why),
        Clock::now(limits),
    )
    .await?;
    urgent::dismiss(db, urgent_id).await
}

/// Asks the failed run again on the item's next prompt.
pub async fn queue(db: &Db, item_id: &str, run: i64) -> AppResult<()> {
    meta::set_text(db, &pending(item_id), &run.to_string()).await
}

/// The failed run to ask again on the item's next prompt, if a retry was asked for. Taken, so it's
/// asked once; one whose run has expired since is dropped and the agent carries on as usual.
pub async fn take(db: &Db, item_id: &str) -> AppResult<Option<Asked>> {
    let key = pending(item_id);
    let Some(run) = meta::text(db, &key).await?.and_then(|r| r.parse().ok()) else {
        return Ok(None);
    };
    meta::forget(db, &key).await?;
    Ok(run_prompts::get(db, run).await.ok())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::domain::item::{ItemInput, Stage};
    use crate::domain::run::Run;
    use crate::domain::urgent::Urgent;
    use crate::store::{items, runs};
    use crate::work;

    async fn failed_run(db: &Db) -> i64 {
        let input = ItemInput {
            id: "B-1".to_owned(),
            title: "t".to_owned(),
            state_label: String::new(),
            stage: Stage::InProgress,
            status: None,
            area: "work".to_owned(),
            link: None,
        };
        work::save(db, input, Utc::now()).await.unwrap();
        let run = Run {
            id: 0,
            schedule_id: None,
            what: "Answer".to_owned(),
            target: "B-1".to_owned(),
            node: "this-mac".to_owned(),
            started_at: Utc::now(),
            finished_at: None,
            outcome: None,
            summary: None,
        };
        let id = runs::start(db, &run).await.unwrap();
        run_prompts::keep(db, id, "you said: ship it")
            .await
            .unwrap();
        id
    }

    fn card(run_id: Option<i64>) -> Urgent {
        Urgent {
            id: 0,
            title: "Your agent couldn't run on B-1".to_owned(),
            detail: String::new(),
            area: "work".to_owned(),
            raised_at: Utc::now(),
            schedule_id: None,
            run_id,
        }
    }

    #[tokio::test]
    async fn a_retry_asks_the_failed_run_again_once() {
        let db = Db::in_memory().await;
        let run = failed_run(&db).await;
        let id = urgent::raise(&db, &card(Some(run))).await.unwrap();
        request(&db, id, &TurnSettings::default()).await.unwrap();

        assert!(items::get(&db, "B-1").await.unwrap().is_turn(Turn::Agent));
        assert!(urgent::list(&db).await.unwrap().is_empty());
        let asked = take(&db, "B-1").await.unwrap().unwrap();
        assert_eq!(
            (asked.what.as_str(), asked.prompt.as_str()),
            ("Answer", "you said: ship it")
        );
        assert!(take(&db, "B-1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn an_alert_without_a_run_cant_be_retried() {
        let db = Db::in_memory().await;
        let id = urgent::raise(&db, &card(None)).await.unwrap();
        assert!(request(&db, id, &TurnSettings::default()).await.is_err());
        assert_eq!(urgent::list(&db).await.unwrap().len(), 1);
    }
}
