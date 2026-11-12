//! Problems that stop work moving must stay in sight while they last: one urgent card per problem,
//! raised once it has lasted `after`, raised again if dismissed while it still holds, and cleared
//! the moment it's gone.

use chrono::{DateTime, Duration, Utc};

use crate::domain::urgent::Urgent;
use crate::error::AppResult;
use crate::store::{Db, meta, urgent};

/// One kind of lasting problem, named by `key` in the meta table.
#[derive(Debug, Clone)]
pub struct Lasting {
    pub key: String,
    pub after: Duration,
}

/// What its card says. `since` is when the problem started.
pub struct Card<'a> {
    pub title: &'a str,
    pub detail: &'a (dyn Fn(DateTime<Utc>) -> String + Sync),
    pub area: &'a str,
}

impl Lasting {
    fn since_key(&self) -> String {
        format!("{}.since", self.key)
    }

    fn card_key(&self) -> String {
        format!("{}.card", self.key)
    }

    async fn card(&self, db: &Db) -> AppResult<Option<i64>> {
        Ok(meta::text(db, &self.card_key())
            .await?
            .and_then(|c| c.parse().ok()))
    }

    /// The problem is gone: its card goes too.
    pub async fn clear(&self, db: &Db) -> AppResult<()> {
        if let Some(id) = self.card(db).await? {
            urgent::dismiss(db, id).await?;
        }
        meta::forget(db, &self.card_key()).await?;
        meta::forget(db, &self.since_key()).await
    }

    /// The problem holds now. Returns whether a card was raised.
    pub async fn holds(&self, db: &Db, card: &Card<'_>) -> AppResult<bool> {
        let now = Utc::now();
        let Some(since) = meta::time(db, &self.since_key()).await? else {
            meta::set_time(db, &self.since_key(), now).await?;
            return Ok(false);
        };
        if now - since < self.after {
            return Ok(false);
        }
        let shown = urgent::list(db).await?;
        if let Some(id) = self.card(db).await?
            && shown.iter().any(|u| u.id == id)
        {
            return Ok(false);
        }
        let raised = Urgent {
            id: 0,
            title: card.title.to_owned(),
            detail: (card.detail)(since),
            area: card.area.to_owned(),
            raised_at: now,
            schedule_id: None,
            run_id: None,
        };
        let id = urgent::raise(db, &raised).await?;
        meta::set_text(db, &self.card_key(), &id.to_string()).await?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_lasting_problem_stays_raised_until_it_is_gone() {
        let db = Db::in_memory().await;
        let lasting = Lasting {
            key: "test".to_owned(),
            after: Duration::minutes(5),
        };
        let detail = |_| "it broke".to_owned();
        let card = Card {
            title: "Broken",
            detail: &detail,
            area: "",
        };
        assert!(!lasting.holds(&db, &card).await.unwrap());
        assert!(urgent::list(&db).await.unwrap().is_empty());
        let long_ago = Utc::now() - Duration::minutes(10);
        meta::set_time(&db, "test.since", long_ago).await.unwrap();
        assert!(lasting.holds(&db, &card).await.unwrap());
        assert!(!lasting.holds(&db, &card).await.unwrap());
        let first = urgent::list(&db).await.unwrap().first().unwrap().id;
        urgent::dismiss(&db, first).await.unwrap();
        assert!(lasting.holds(&db, &card).await.unwrap());
        lasting.clear(&db).await.unwrap();
        assert!(urgent::list(&db).await.unwrap().is_empty());
    }
}
