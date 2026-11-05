//! The `[turns]` section of `quarterdeck.toml`: how long each turn lasts before something happens.

use chrono::Duration;
use serde::{Deserialize, Serialize};

use crate::domain::turn::Turn;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct TurnSettings {
    /// The agent's turn, counted from when it got it. Handing the turn to itself doesn't extend it.
    pub agent_hours: i64,
    /// Between reminders while it's the user's turn.
    pub you_remind_hours: i64,
    /// Waiting on reviewers, CI or another team before the turn comes back.
    pub others_hours: i64,
    /// How often work parked until something happens (rather than until a date) is checked.
    pub later_hours: i64,
    /// Who gets the turn when others' wait runs out, unless the item says otherwise: `agent`, to
    /// check whether it's still waiting, or `you`. Never another wait.
    pub others_then: Turn,
    /// How long an agent session working outside Quarterdeck can go quiet before it's prompted.
    pub quiet_minutes: i64,
    /// How many times in a row a failed agent run is tried again, in a new session, before the
    /// turn goes to the user.
    pub failure_retries: u32,
}

impl TurnSettings {
    /// `quiet_minutes` as a length, or an error when it's too long to be one.
    pub fn quiet(&self) -> AppResult<Duration> {
        let minutes = self.quiet_minutes;
        Duration::try_minutes(minutes).ok_or_else(|| {
            AppError::BadInput(format!("[turns] quiet_minutes {minutes} is too long"))
        })
    }
}

impl Default for TurnSettings {
    fn default() -> Self {
        Self {
            agent_hours: 4,
            you_remind_hours: 24,
            others_hours: 24,
            later_hours: 24,
            others_then: Turn::Agent,
            quiet_minutes: 45,
            failure_retries: 2,
        }
    }
}

/// What `set_turn_limits` changes. Fields left `None` keep their current value.
#[derive(Debug, Clone, Copy, Default)]
pub struct TurnChange {
    pub agent_hours: Option<i64>,
    pub you_remind_hours: Option<i64>,
    pub others_hours: Option<i64>,
    pub later_hours: Option<i64>,
    pub others_then: Option<Turn>,
    pub quiet_minutes: Option<i64>,
    pub failure_retries: Option<u32>,
}

/// The longest anything but the user's turn may go before it's looked at again. Not a setting: it's
/// the guarantee that nothing waits unchecked for more than a day.
pub const LONGEST_CHECK_HOURS: i64 = 24;

/// The most a failed run is tried again in a row.
const MAX_RETRIES: u32 = 10;

/// The longest the user may go between reminders: a year.
const MAX_TURN_HOURS: i64 = 24 * 365;

impl TurnChange {
    /// Refuses lengths that are zero or negative, waits over a day and reminders over a year.
    pub fn check(&self) -> AppResult<()> {
        let waits = [self.agent_hours, self.others_hours, self.later_hours];
        if !waits
            .iter()
            .flatten()
            .all(|h| (1..=LONGEST_CHECK_HOURS).contains(h))
        {
            return Err(AppError::BadInput(format!(
                "agent_hours, others_hours and later_hours must be between 1 and \
                 {LONGEST_CHECK_HOURS}: nothing waits unchecked for more than a day"
            )));
        }
        if self
            .you_remind_hours
            .is_some_and(|h| !(1..=MAX_TURN_HOURS).contains(&h))
        {
            return Err(AppError::BadInput(format!(
                "you_remind_hours must be between 1 and {MAX_TURN_HOURS}"
            )));
        }
        if matches!(self.others_then, Some(Turn::Others | Turn::Later)) {
            return Err(AppError::BadInput(
                "others_then must be \"you\" or \"agent\"".to_owned(),
            ));
        }
        if self.failure_retries.is_some_and(|r| r > MAX_RETRIES) {
            return Err(AppError::BadInput(format!(
                "failure_retries must be at most {MAX_RETRIES}"
            )));
        }
        let max_minutes = MAX_TURN_HOURS * 60;
        if self
            .quiet_minutes
            .is_some_and(|m| !(1..=max_minutes).contains(&m))
        {
            return Err(AppError::BadInput(format!(
                "quiet_minutes must be between 1 and {max_minutes}"
            )));
        }
        Ok(())
    }
}
