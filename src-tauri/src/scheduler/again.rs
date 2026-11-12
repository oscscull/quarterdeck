//! A failed run is tried again by itself, in a new session, up to `[turns] failure_retries` times
//! in a row before the user hears of it: most failures are blips, and a session that broke is left
//! behind rather than resumed.

use super::failed::Failure;
use crate::config::turns::TurnSettings;
use crate::domain::event::Change;
use crate::domain::turn::Turn;
use crate::error::AppResult;
use crate::retry;
use crate::store::{Db, watch};
use crate::turns::{self, Clock, Pass};

/// Starts the turn reason of every automatic retry, so they can be counted.
const MARK: &str = "trying again after a failed run";

/// Tries the failed run again if the item has retries left; false once they're used up. Any other
/// pass of the turn in between starts the count again.
pub async fn try_again(db: &Db, f: &Failure<'_>, limits: &TurnSettings) -> AppResult<bool> {
    let other_pass = |c: &Change| matches!(c, Change::Turn { .. });
    let tries = turns::history::streak(db, f.item_id, MARK, other_pass).await?;
    let budget = usize::try_from(limits.failure_retries).unwrap_or(usize::MAX);
    if tries >= budget {
        return Ok(false);
    }
    watch::release(db, f.item_id).await?;
    if let Some(run) = f.run {
        retry::queue(db, f.item_id, run).await?;
    }
    let why = format!(
        "{MARK} ({} of {budget}), in a new session: {}",
        tries + 1,
        f.detail()
    );
    turns::pass(
        db,
        f.item_id,
        Pass::to(Turn::Agent, why),
        Clock::now(limits),
    )
    .await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::domain::item::{ItemInput, Stage};
    use crate::store::items;
    use crate::work;

    async fn started(db: &Db, limits: &TurnSettings) {
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
        let pass = Pass::to(Turn::Agent, "you said: fix it");
        turns::pass(db, "B-1", pass, Clock::now(limits))
            .await
            .unwrap();
    }

    fn failure() -> Failure<'static> {
        Failure {
            item_id: "B-1",
            what: "Carry on",
            detail: "boom",
            run: None,
        }
    }

    #[tokio::test]
    async fn failures_are_retried_a_few_times_then_handed_on() {
        let db = Db::in_memory().await;
        let limits = TurnSettings::default();
        started(&db, &limits).await;
        for _ in 0..limits.failure_retries {
            assert!(try_again(&db, &failure(), &limits).await.unwrap());
            assert!(items::get(&db, "B-1").await.unwrap().is_turn(Turn::Agent));
        }
        assert!(!try_again(&db, &failure(), &limits).await.unwrap());
        let pass = Pass::to(Turn::Agent, "the agent carried on");
        turns::pass(&db, "B-1", pass, Clock::now(&limits))
            .await
            .unwrap();
        assert!(try_again(&db, &failure(), &limits).await.unwrap());
    }
}
