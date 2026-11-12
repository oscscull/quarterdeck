//! Where an item is in its life, and whose move it is. Every open item, started or not, is someone's
//! turn, and each turn carries what it needs: the user's says what's wanted from them, others' who
//! it waits on, a later one why it can't start yet and what wakes it. There is no open state
//! without a turn, so there is nowhere for work to sit unseen. Stored and sent as flat fields
//! (`LifeFields`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::item::Stage;
use super::trigger::OnEvent;
use super::turn::Turn;

/// What waits on others says when nobody named who.
const SOMEONE: &str = "others";

/// What the user's turn on work nobody has started needs.
pub const NOT_STARTED: &str = "Not started: start it, park it until it can, or close it";

/// What the user's turn on started work stored without a need asks.
const NEXT: &str = "What happens next?";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "LifeFields", into = "LifeFields")]
pub enum Life {
    Open(Open),
    Done,
}

/// The stages of open work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Not started yet. Still someone's turn: the user's to decide, or a wait or parking that says
    /// why it can't start.
    UpNext,
    InProgress,
    InReview,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Open {
    pub phase: Phase,
    pub whose: Whose,
    /// When the turn runs out. Unset only on work stored before it had one; the next tick sets it.
    pub until: Option<DateTime<Utc>>,
}

/// Whose move it is, with what that turn needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Whose {
    You {
        need: String,
    },
    Agent,
    Others {
        waiting_on: String,
        then: Option<Turn>,
    },
    Later {
        then: Option<Turn>,
        wake: Vec<OnEvent>,
        /// Why it can't start yet and what to do once it's back, said again when it wakes.
        note: Option<String>,
    },
}

impl Whose {
    pub const fn turn(&self) -> Turn {
        match self {
            Self::You { .. } => Turn::You,
            Self::Agent => Turn::Agent,
            Self::Others { .. } => Turn::Others,
            Self::Later { .. } => Turn::Later,
        }
    }
}

impl Phase {
    pub const fn stage(self) -> Stage {
        match self {
            Self::UpNext => Stage::UpNext,
            Self::InProgress => Stage::InProgress,
            Self::InReview => Stage::InReview,
        }
    }

    pub const fn of(stage: Stage) -> Option<Self> {
        match stage {
            Stage::UpNext => Some(Self::UpNext),
            Stage::InProgress => Some(Self::InProgress),
            Stage::InReview => Some(Self::InReview),
            Stage::Done => None,
        }
    }
}

impl Life {
    /// Open work at `phase` whose turn is `whose`.
    pub const fn open_as(phase: Phase, whose: Whose, until: Option<DateTime<Utc>>) -> Self {
        Self::Open(Open {
            phase,
            whose,
            until,
        })
    }

    /// The user's turn on work nobody has started: the backlog's only form.
    pub fn not_started() -> Self {
        let need = NOT_STARTED.to_owned();
        Self::open_as(Phase::UpNext, Whose::You { need }, None)
    }

    /// Not started and nobody has decided anything about it yet, whatever its clock says.
    pub fn untouched(&self) -> bool {
        matches!(
            self.open(),
            Some(Open { phase: Phase::UpNext, whose: Whose::You { need }, .. }) if need == NOT_STARTED
        )
    }

    pub const fn stage(&self) -> Stage {
        match self {
            Self::Open(o) => o.phase.stage(),
            Self::Done => Stage::Done,
        }
    }

    pub const fn open(&self) -> Option<&Open> {
        match self {
            Self::Open(o) => Some(o),
            Self::Done => None,
        }
    }

    pub const fn whose(&self) -> Option<&Whose> {
        match self {
            Self::Open(o) => Some(&o.whose),
            Self::Done => None,
        }
    }

    pub const fn turn(&self) -> Option<Turn> {
        match self {
            Self::Open(o) => Some(o.whose.turn()),
            Self::Done => None,
        }
    }

    pub fn until(&self) -> Option<DateTime<Utc>> {
        self.open().and_then(|o| o.until)
    }
}

/// How a life is stored and sent: the item's stage and turn fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifeFields {
    pub stage: Stage,
    #[serde(default)]
    pub turn: Option<Turn>,
    #[serde(default)]
    pub turn_until: Option<DateTime<Utc>>,
    /// The user's turn: what's needed. A later turn: its note.
    #[serde(default)]
    pub need: Option<String>,
    #[serde(default)]
    pub waiting_on: Option<String>,
    #[serde(default)]
    pub then_turn: Option<Turn>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wake: Vec<OnEvent>,
}

/// Open work stored without a turn is the user's, saying what it needs: the safe side, as it
/// shows on Today.
fn whose(phase: Phase, f: LifeFields) -> Whose {
    match f.turn.unwrap_or(Turn::You) {
        Turn::You => {
            let fallback = if phase == Phase::UpNext {
                NOT_STARTED
            } else {
                NEXT
            };
            let need = f.need.filter(|n| !n.trim().is_empty());
            Whose::You {
                need: need.unwrap_or_else(|| fallback.to_owned()),
            }
        }
        Turn::Agent => Whose::Agent,
        Turn::Others => Whose::Others {
            waiting_on: f.waiting_on.unwrap_or_else(|| SOMEONE.to_owned()),
            then: f.then_turn,
        },
        Turn::Later => Whose::Later {
            then: f.then_turn,
            wake: f.wake,
            note: f.need,
        },
    }
}

impl From<LifeFields> for Life {
    fn from(f: LifeFields) -> Self {
        let Some(phase) = Phase::of(f.stage) else {
            return Self::Done;
        };
        let until = f.turn_until;
        Self::Open(Open {
            phase,
            whose: whose(phase, f),
            until,
        })
    }
}

fn whose_fields(whose: Whose, f: &mut LifeFields) {
    f.turn = Some(whose.turn());
    match whose {
        Whose::You { need } => f.need = Some(need),
        Whose::Agent => {}
        Whose::Others { waiting_on, then } => {
            f.waiting_on = Some(waiting_on);
            f.then_turn = then;
        }
        Whose::Later { then, wake, note } => {
            f.then_turn = then;
            f.wake = wake;
            f.need = note;
        }
    }
}

impl From<Life> for LifeFields {
    fn from(life: Life) -> Self {
        let mut f = Self {
            stage: life.stage(),
            turn: None,
            turn_until: None,
            need: None,
            waiting_on: None,
            then_turn: None,
            wake: Vec::new(),
        };
        if let Life::Open(o) = life {
            f.turn_until = o.until;
            whose_fields(o.whose, &mut f);
        }
        f
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(life: Life) -> Life {
        Life::from(LifeFields::from(life))
    }

    #[test]
    fn every_life_survives_its_fields() {
        let lives = [
            Life::not_started(),
            Life::Done,
            Life::open_as(
                Phase::InReview,
                Whose::You {
                    need: "Merge it?".to_owned(),
                },
                Some(Utc::now()),
            ),
            Life::open_as(
                Phase::InProgress,
                Whose::Others {
                    waiting_on: "CI".to_owned(),
                    then: Some(Turn::Agent),
                },
                None,
            ),
            Life::open_as(
                Phase::UpNext,
                Whose::Later {
                    then: Some(Turn::Agent),
                    wake: Vec::new(),
                    note: Some("phase 1 must ship first: start once it's merged".to_owned()),
                },
                Some(Utc::now()),
            ),
        ];
        for life in lives {
            assert_eq!(round_trip(life.clone()), life);
        }
    }

    fn stored(stage: Stage, turn: Option<Turn>) -> LifeFields {
        LifeFields {
            stage,
            turn,
            turn_until: None,
            need: None,
            waiting_on: None,
            then_turn: None,
            wake: Vec::new(),
        }
    }

    #[test]
    fn open_work_stored_without_a_turn_is_the_users_with_a_need() {
        assert_eq!(Life::from(stored(Stage::UpNext, None)), Life::not_started());
        let started = Life::from(stored(Stage::InProgress, None));
        assert!(matches!(started.whose(), Some(Whose::You { need }) if need == NEXT));
        assert_eq!(
            Life::from(stored(Stage::Done, Some(Turn::Agent))),
            Life::Done
        );
    }
}
