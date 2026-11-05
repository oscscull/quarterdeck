//! Parking: putting off work that can't start or go on yet. It is never for work that merely isn't
//! due: work that can start, starts. Parking always says why (`why`), and is one of two kinds:
//!
//! - until a date: it comes back then (or sooner, on a `wake` event) to the user, or the agent if
//!   the parking says so. Any date is fine, as it is a concrete rule.
//! - until something happens: `wake` events bring it back as soon as they fire, and in any case
//!   the agent looks again every `later_hours` (at most a day), starting it if it can or parking
//!   it again if not. A condition nobody checks could wait forever; this one can't.
//!
//! Either way it's a later turn, so it leaves Today for the Parked list with a countdown, and its
//! note becomes what the user or agent is told to do when it's back.

use chrono::{DateTime, Utc};

use crate::domain::fired::known;
use crate::domain::life::{Life, Whose};
use crate::domain::trigger::OnEvent;
use crate::domain::turn::Turn;
use crate::error::{AppError, AppResult};
use crate::store::{Db, items};
use crate::turns::{self, Clock, Pass};

/// What a parked item waits for, and why.
#[derive(Debug, Clone, Default)]
pub struct Parking {
    /// When it comes back. Unset, it waits for something to happen, which the agent checks.
    pub until: Option<DateTime<Utc>>,
    /// Events that bring it back sooner.
    pub wake: Vec<OnEvent>,
    /// Why it can't start or go on yet, and what to do once it's back.
    pub why: Option<String>,
    /// Who picks it up when it's back from a date: the user (unset) or the agent.
    pub then: Option<Turn>,
}

fn bad(text: &str) -> AppError {
    AppError::BadInput(text.to_owned())
}

fn check(life: &Life, parking: &Parking, now: DateTime<Utc>) -> AppResult<()> {
    if *life == Life::Done {
        return Err(bad("it's done"));
    }
    if note(parking).is_none() {
        return Err(bad(
            "say in `why` what stops it starting now and what to do once it can; work that can \
             start isn't parked, and work that isn't due yet can still start",
        ));
    }
    if parking.until.is_some_and(|u| u <= now) {
        return Err(bad("`until` has passed: pick a time to come back"));
    }
    if matches!(parking.then, Some(Turn::Others | Turn::Later)) {
        return Err(bad(
            "it comes back to the user or the agent (`then`: you or agent)",
        ));
    }
    if parking.until.is_none() && parking.then == Some(Turn::You) {
        return Err(bad(
            "with no date it waits for something to happen, which the agent checks daily: give \
             `until` to bring it back to the user on a date",
        ));
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

/// Who gets it once it's back: as asked on a date, else the agent, to check.
const fn then(parking: &Parking) -> Turn {
    match (parking.until, parking.then) {
        (Some(_), Some(Turn::Agent)) | (None, _) => Turn::Agent,
        (Some(_), _) => Turn::You,
    }
}

pub async fn park(db: &Db, id: &str, parking: Parking, clock: Clock<'_>) -> AppResult<()> {
    let item = items::get(db, id).await?;
    check(&item.life, &parking, clock.now)?;
    let note = note(&parking).map(str::to_owned);
    let why = note
        .as_deref()
        .map_or_else(|| "parked".to_owned(), |w| format!("parked: {w}"));
    let pass = Pass {
        until: parking.until,
        then: Some(then(&parking)),
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
        item.life.open().map(|a| &a.whose),
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
            why: Some("the API it calls ships tomorrow".to_owned()),
            ..Parking::default()
        }
    }

    #[tokio::test]
    async fn parked_work_waits_on_a_later_turn_until_brought_back() {
        let db = Db::in_memory().await;
        let limits = TurnSettings::default();
        save(&db, Stage::InProgress).await;
        let unexplained = Parking {
            why: None,
            ..until_tomorrow()
        };
        let parked = park(&db, "app#7", unexplained, Clock::now(&limits)).await;
        assert!(parked.is_err());
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
                why:
                    "back from parking (traffic should be 0: remove the old route): start it if it \
                      can start now, else park it again"
                        .to_owned(),
            }
        );
    }

    #[tokio::test]
    async fn work_waiting_on_something_is_checked_by_the_agent_within_a_day() {
        let db = Db::in_memory().await;
        let limits = TurnSettings {
            later_hours: 24 * 30,
            ..TurnSettings::default()
        };
        save(&db, Stage::UpNext).await;
        let parking = Parking {
            why: Some("phase 1 (app#6) must merge first".to_owned()),
            ..Parking::default()
        };
        park(&db, "app#7", parking, Clock::now(&limits))
            .await
            .unwrap();
        let item = items::get(&db, "app#7").await.unwrap();
        let back = item.life.until().unwrap();
        assert!(back <= Utc::now() + chrono::Duration::hours(24));
        let due = turns::due::due(&item, turns::due::Work::Idle, &limits, back);
        assert!(matches!(
            due,
            turns::due::Due::PassTo {
                to: Turn::Agent,
                ..
            }
        ));
        let to_you = Parking {
            then: Some(Turn::You),
            why: Some("phase 1 must merge first".to_owned()),
            ..Parking::default()
        };
        assert!(
            park(&db, "app#7", to_you, Clock::now(&limits))
                .await
                .is_err()
        );
    }
}
