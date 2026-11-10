//! Work handed to the user only because the agent couldn't be reached isn't theirs to do: it goes
//! back to the agent, with its failed run asked again if its alert still knows which, and the
//! alert cleared.

use super::failed;
use crate::claude::reach;
use crate::config::turns::TurnSettings;
use crate::domain::item::WorkItem;
use crate::domain::life::{Life, Open, Whose};
use crate::domain::turn::Turn;
use crate::error::AppResult;
use crate::retry;
use crate::store::{Db, items, urgent};
use crate::turns::{self, Clock, Pass};

const BACK: &str = "the agent can run again, so it carries on";

fn stranded(item: &WorkItem) -> bool {
    let Life::Open(Open {
        whose: Whose::You { need },
        ..
    }) = &item.life
    else {
        return false;
    };
    need.starts_with(failed::COULDNT_RUN) && reach::unreached(need)
}

/// Gives every stranded item back to the agent. Called while the agent can run.
pub async fn resume(db: &Db, limits: &TurnSettings) -> AppResult<()> {
    let waiting: Vec<WorkItem> = items::list(db).await?;
    for item in waiting.iter().filter(|i| stranded(i)) {
        let title = failed::alert_title(&item.id);
        let mut retried = false;
        for card in urgent::list(db).await? {
            if card.title != title {
                continue;
            }
            // A retry passes the turn itself; a card with no run to retry is only cleared.
            retried = retried || retry::request(db, card.id, limits).await.is_ok();
            urgent::dismiss(db, card.id).await?;
        }
        if !retried {
            let pass = Pass::to(Turn::Agent, BACK);
            turns::pass(db, &item.id, pass, Clock::now(limits)).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::domain::item::{ItemInput, Stage};
    use crate::domain::urgent::Urgent;
    use crate::work;

    async fn handed_back(db: &Db, id: &str, why: &str) {
        let input = ItemInput {
            id: id.to_owned(),
            title: "t".to_owned(),
            state_label: String::new(),
            stage: Stage::InProgress,
            status: None,
            area: "work".to_owned(),
            link: None,
        };
        work::save(db, input, Utc::now()).await.unwrap();
        let limits = TurnSettings::default();
        turns::pass(db, id, Pass::to(Turn::You, why), Clock::now(&limits))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn only_work_stranded_by_an_outage_goes_back_to_the_agent() {
        let db = Db::in_memory().await;
        let logged_out = "your agent couldn't run (Answer): claude failed: Not logged in";
        handed_back(&db, "B-1", logged_out).await;
        handed_back(&db, "B-2", "your agent couldn't run (Answer): boom").await;
        let card = Urgent {
            id: 0,
            title: failed::alert_title("B-1"),
            detail: String::new(),
            area: "work".to_owned(),
            raised_at: Utc::now(),
            schedule_id: None,
            run_id: None,
        };
        urgent::raise(&db, &card).await.unwrap();

        resume(&db, &TurnSettings::default()).await.unwrap();

        assert!(items::get(&db, "B-1").await.unwrap().is_turn(Turn::Agent));
        assert!(items::get(&db, "B-2").await.unwrap().is_turn(Turn::You));
        assert!(urgent::list(&db).await.unwrap().is_empty());
    }
}
