//! Parking: putting started work off until something happens, a time or an event, whichever comes
//! first. It's a later turn, so it leaves Today and comes back when it wakes, to the user or, if
//! the parking says so, the agent, with the reason it was parked as what to do now. Nothing
//! parks for good (work nobody will pick up is closed instead), and unstarted work isn't parked:
//! the backlog is already off Today.

use chrono::{DateTime, Utc};

use crate::domain::fired::known;
use crate::domain::life::{Life, Whose};
use crate::domain::trigger::OnEvent;
use crate::domain::turn::Turn;
use crate::error::{AppError, AppResult};
use crate::store::{Db, items};
use crate::turns::{self, Clock, Pass};

/// What a parked item waits for, and the user's reason if they gave one.
#[derive(Debug, Clone, Default)]
pub struct Parking {
    /// When it comes back. With only events, the configured later length is the backstop.
    pub until: Option<DateTime<Utc>>,
    /// Events that bring it back sooner.
    pub wake: Vec<OnEvent>,
    /// Why it waits and what to do once it's back.
    pub why: Option<String>,
    /// Who picks it up when it's back: the user (unset) or the agent.
    pub then: Option<Turn>,
}

fn bad(text: &str) -> AppError {
    AppError::BadInput(text.to_owned())
}

fn check(life: &Life, parking: &Parking) -> AppResult<()> {
    match life {
        Life::Unstarted => {
            return Err(bad(
                "it isn't started, so it's already off Today: leave it up next, or close it",
            ));
        }
        Life::Done => return Err(bad("it's done")),
        Life::Active(_) => {}
    }
    if parking.until.is_none() && parking.wake.is_empty() {
        return Err(bad(
            "say what it waits for: a time (`until`), events (`wake`), or both",
        ));
    }
    if matches!(parking.then, Some(Turn::Others | Turn::Later)) {
        return Err(bad(
            "it comes back to the user or the agent (`then`: you or agent)",
        ));
    }
    if parking.then == Some(Turn::Agent) && note(parking).is_none() {
        return Err(bad("say in `why` what the agent should do once it's back"));
    }
    if let Some(e) = parking.wake.iter().find(|e| !known(&e.on)) {
        return Err(AppError::BadInput(format!(
            "unknown event '{}' (see list_event_kinds)",
            e.on
        )));
    }
    Ok(())
}

fn note(parking: &Parking) -> Option<&str> {
    parking
        .why
        .as_deref()
        .map(str::trim)
        .filter(|w| !w.is_empty())
}

pub async fn park(db: &Db, id: &str, parking: Parking, clock: Clock<'_>) -> AppResult<()> {
    let item = items::get(db, id).await?;
    check(&item.life, &parking)?;
    let note = note(&parking).map(str::to_owned);
    let why = note
        .as_deref()
        .map_or_else(|| "parked".to_owned(), |w| format!("parked: {w}"));
    let pass = Pass {
        until: parking.until,
        then: Some(parking.then.unwrap_or(Turn::You)),
        wake: parking.wake,
        need: note,
        ..Pass::to(Turn::Later, why)
    };
    turns::pass(db, id, pass, clock).await
}

/// Parked work, highest priority first: everything on a later turn.
pub async fn list(db: &Db) -> AppResult<Vec<crate::domain::item::WorkItem>> {
    let mut all = items::list(db).await?;
    all.retain(|i| i.is_turn(Turn::Later));
    Ok(all)
}

/// Brings parked work back to the user now.
pub async fn unpark(db: &Db, id: &str, clock: Clock<'_>) -> AppResult<()> {
    let item = items::get(db, id).await?;
    if !matches!(
        item.life.active().map(|a| &a.whose),
        Some(Whose::Later { .. })
    ) {
        return Ok(());
    }
    let pass = Pass::to(Turn::You, "unparked").needing("Unparked: what happens next?");
    turns::pass(db, id, pass, clock).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::turns::TurnSettings;
    use crate::domain::item::{ItemInput, Stage};
    use crate::work;

    async fn save(db: &Db, stage: Stage) {
        let input = ItemInput {
            id: "app#7".to_owned(),
            title: "Old idea".to_owned(),
            state_label: String::new(),
            stage,
            status: None,
            area: "projects".to_owned(),
            link: None,
        };
        work::save(db, input, Utc::now()).await.unwrap();
    }

    fn until_tomorrow() -> Parking {
        Parking {
            until: Some(Utc::now() + chrono::Duration::days(1)),
            ..Parking::default()
        }
    }

    #[tokio::test]
    async fn parked_work_waits_on_a_later_turn_until_brought_back() {
        let db = Db::in_memory().await;
        let limits = TurnSettings::default();
        save(&db, Stage::InProgress).await;
        let forever = park(&db, "app#7", Parking::default(), Clock::now(&limits)).await;
        assert!(forever.is_err());
        park(&db, "app#7", until_tomorrow(), Clock::now(&limits))
            .await
            .unwrap();
        assert!(items::get(&db, "app#7").await.unwrap().is_turn(Turn::Later));
        unpark(&db, "app#7", Clock::now(&limits)).await.unwrap();
        assert!(items::get(&db, "app#7").await.unwrap().is_turn(Turn::You));
    }

    #[tokio::test]
    async fn parked_work_comes_back_to_the_agent_with_what_to_do() {
        let db = Db::in_memory().await;
        let limits = TurnSettings::default();
        save(&db, Stage::InProgress).await;
        let silent = Parking {
            then: Some(Turn::Agent),
            ..until_tomorrow()
        };
        assert!(
            park(&db, "app#7", silent, Clock::now(&limits))
                .await
                .is_err()
        );
        let parking = Parking {
            then: Some(Turn::Agent),
            why: Some("traffic should be 0: remove the old route".to_owned()),
            ..until_tomorrow()
        };
        park(&db, "app#7", parking, Clock::now(&limits))
            .await
            .unwrap();
        let item = items::get(&db, "app#7").await.unwrap();
        let later = Clock {
            limits: &limits,
            now: Utc::now() + chrono::Duration::days(2),
        };
        let due = turns::due::due(&item, turns::due::Work::Idle, &limits, later.now);
        assert_eq!(
            due,
            turns::due::Due::PassTo {
                to: Turn::Agent,
                why: "back from parking: traffic should be 0: remove the old route".to_owned(),
            }
        );
    }

    #[tokio::test]
    async fn unstarted_work_is_not_parked() {
        let db = Db::in_memory().await;
        let limits = TurnSettings::default();
        save(&db, Stage::UpNext).await;
        let parked = park(&db, "app#7", until_tomorrow(), Clock::now(&limits)).await;
        assert!(parked.is_err());
    }
}
