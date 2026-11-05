//! Passing an item's turn. It's the only way a turn changes, so every change is in the item's
//! history with its reason.

pub mod due;
pub mod history;
mod pass;
pub mod work;

pub use pass::Pass;

use chrono::{DateTime, TimeDelta, Utc};

use crate::config::turns::LONGEST_CHECK_HOURS;
use crate::config::turns::TurnSettings;
use crate::domain::event::Change;
use crate::domain::life::{Life, Open, Phase, Whose};
use crate::domain::turn::Turn;
use crate::error::{AppError, AppResult};
use crate::store::{Db, events, items};

/// The turn limits, and when a change happens.
#[derive(Debug, Clone, Copy)]
pub struct Clock<'a> {
    pub limits: &'a TurnSettings,
    pub now: DateTime<Utc>,
}

impl<'a> Clock<'a> {
    pub fn now(limits: &'a TurnSettings) -> Self {
        Self {
            limits,
            now: Utc::now(),
        }
    }

    /// When `turn` runs out, starting now. Every turn but the user's is looked at again within
    /// `LONGEST_CHECK_HOURS`, whatever the config says. Fails on a length too large to add.
    pub fn deadline(self, turn: Turn) -> AppResult<DateTime<Utc>> {
        let hours = match turn {
            Turn::You => self.limits.you_remind_hours,
            Turn::Agent => self.limits.agent_hours.min(LONGEST_CHECK_HOURS),
            Turn::Others => self.limits.others_hours.min(LONGEST_CHECK_HOURS),
            Turn::Later => self.limits.later_hours.min(LONGEST_CHECK_HOURS),
        };
        self.after(hours.max(1))
    }

    /// The latest anything waiting may go before it's looked at again.
    pub fn check_by(self) -> AppResult<DateTime<Utc>> {
        self.after(LONGEST_CHECK_HOURS)
    }

    fn after(self, hours: i64) -> AppResult<DateTime<Utc>> {
        TimeDelta::try_hours(hours)
            .and_then(|d| self.now.checked_add_signed(d))
            .ok_or_else(|| AppError::BadInput(format!("[turns] {hours} hours is too long")))
    }
}

/// Whether a pass keeps the clock already running: an agent handing the turn to itself, so it can't
/// keep itself going forever, and a wait that only changes who it's on (a PR going from CI to
/// reviewers and back), so a wait can't be renewed by its own churn.
const fn keeps_clock(old: &Open, to: Turn) -> bool {
    matches!(
        (&old.whose, to),
        (Whose::Agent, Turn::Agent) | (Whose::Others { .. }, Turn::Others)
    )
}

/// The deadline after a pass. Only the user's turn (which is on Today) and parking until a date
/// may run longer than `LONGEST_CHECK_HOURS`: every other turn is looked at again within a day.
fn until_after(old: &Open, pass: &Pass, clock: Clock<'_>) -> AppResult<DateTime<Utc>> {
    let kept = old.until.filter(|_| keeps_clock(old, pass.to));
    let until = pass
        .until
        .or(kept)
        .map_or_else(|| clock.deadline(pass.to), Ok)?;
    if matches!(pass.to, Turn::Agent | Turn::Others) {
        return Ok(until.min(clock.check_by()?));
    }
    Ok(until)
}

/// Hands the turn on. Giving the agent the turn on work not started starts it; done work has no
/// turn, so nothing changes.
pub async fn pass(db: &Db, item_id: &str, pass: Pass, clock: Clock<'_>) -> AppResult<()> {
    let mut item = items::get(db, item_id).await?;
    let Life::Open(old) = &item.life else {
        return Ok(());
    };
    let until = Some(until_after(old, &pass, clock)?);
    let started = old.phase == Phase::UpNext && pass.to == Turn::Agent;
    let phase = if started {
        Phase::InProgress
    } else {
        old.phase
    };
    item.life = Life::open_as(phase, pass.whose(), until);
    if started {
        item.state_since = clock.now;
    }
    items::save(db, &item).await?;
    if started {
        let stage = Change::Stage {
            stage: phase.stage(),
        };
        events::record(db, item_id, clock.now, &stage).await?;
    }
    let change = Change::Turn {
        turn: pass.to,
        why: pass.why,
    };
    events::record(db, item_id, clock.now, &change).await
}

/// Moves the clock without passing the turn: an item just in flight, or a reminder sent.
pub async fn reset_clock(db: &Db, item_id: &str, until: DateTime<Utc>) -> AppResult<()> {
    let mut item = items::get(db, item_id).await?;
    if let Life::Open(a) = &mut item.life {
        a.until = Some(until);
        items::save(db, &item).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;
    use crate::domain::item::WorkItem;

    async fn open(db: &Db, whose: Whose, until: DateTime<Utc>) {
        let life = Life::open_as(Phase::InReview, whose, Some(until));
        items::save(db, &WorkItem::sample("app#1", life))
            .await
            .unwrap();
    }

    async fn until(db: &Db) -> DateTime<Utc> {
        items::get(db, "app#1").await.unwrap().life.until().unwrap()
    }

    #[tokio::test]
    async fn no_wait_is_set_beyond_a_day() {
        let db = Db::in_memory().await;
        let limits = TurnSettings {
            others_hours: 24 * 365,
            agent_hours: 24 * 365,
            ..TurnSettings::default()
        };
        let clock = Clock::now(&limits);
        open(&db, Whose::Agent, clock.now).await;
        let far = clock.now + Duration::days(3650);
        for to in [Turn::Others, Turn::Agent] {
            let asked = Pass {
                until: Some(far),
                ..Pass::to(to, "wait for years")
            };
            pass(&db, "app#1", asked, clock).await.unwrap();
            assert!(until(&db).await <= clock.check_by().unwrap(), "{to:?}");
        }
    }

    #[tokio::test]
    async fn a_wait_that_only_changes_who_it_is_on_keeps_its_clock() {
        let db = Db::in_memory().await;
        let limits = TurnSettings::default();
        let clock = Clock::now(&limits);
        let soon = clock.now + Duration::hours(2);
        let ci = Whose::Others {
            waiting_on: "CI".to_owned(),
            then: None,
        };
        open(&db, ci, soon).await;
        let reviewers = Pass::to(Turn::Others, "CI passed").waiting_on("Sam");
        pass(&db, "app#1", reviewers, clock).await.unwrap();
        assert_eq!(until(&db).await, soon);
    }
}
