//! Time the agent couldn't run doesn't count against its turns. While it's down the keeper lets no
//! agent's turn run out to the user, and when it's back every agent's turn gets the outage's
//! length added to its clock, so work resumes with the time it had left.

use chrono::{DateTime, Utc};

use crate::domain::life::{Life, Open, Whose};
use crate::error::AppResult;
use crate::store::{Db, items};
use crate::turns;

/// Adds the time since `since` to the clock of every item on the agent's turn.
pub async fn give_back(db: &Db, since: DateTime<Utc>, now: DateTime<Utc>) -> AppResult<()> {
    let lost = now.signed_duration_since(since);
    for item in items::list(db).await? {
        let Life::Open(Open {
            whose: Whose::Agent,
            until: Some(until),
            ..
        }) = item.life
        else {
            continue;
        };
        if let Some(later) = until.checked_add_signed(lost) {
            turns::reset_clock(db, &item.id, later).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;
    use crate::config::turns::TurnSettings;
    use crate::domain::item::{ItemInput, Stage};
    use crate::domain::turn::Turn;
    use crate::turns::{Clock, Pass};
    use crate::work;

    async fn item_on(db: &Db, id: &str, turn: Turn) -> Option<DateTime<Utc>> {
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
        turns::pass(db, id, Pass::to(turn, "why"), Clock::now(&limits))
            .await
            .unwrap();
        until(db, id).await
    }

    async fn until(db: &Db, id: &str) -> Option<DateTime<Utc>> {
        let item = items::get(db, id).await.unwrap();
        item.life.open().and_then(|o| o.until)
    }

    #[tokio::test]
    async fn only_the_agents_turns_get_the_outage_back() {
        let db = Db::in_memory().await;
        let agents = item_on(&db, "B-1", Turn::Agent).await.unwrap();
        let yours = item_on(&db, "B-2", Turn::You).await;
        let now = Utc::now();

        give_back(&db, now - Duration::hours(3), now).await.unwrap();

        assert_eq!(until(&db, "B-1").await, Some(agents + Duration::hours(3)));
        assert_eq!(until(&db, "B-2").await, yours);
    }
}
