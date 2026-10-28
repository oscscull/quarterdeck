//! Where an item is in its life, and whose move it is. An item has a turn exactly while it's
//! active: unstarted work sits in the backlog with nobody on it, started work always has someone on
//! it, and each turn carries what it needs: the user's says what's wanted from them, others' who
//! it waits on, a later one what wakes it. Stored and sent as flat fields (`LifeFields`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::item::Stage;
use super::trigger::OnEvent;
use super::turn::Turn;

/// What waits on others says when nobody named who.
const SOMEONE: &str = "others";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "LifeFields", into = "LifeFields")]
pub enum Life {
    /// Not started: in the backlog, nobody's turn.
    Unstarted,
    Active(Active),
    Done,
}

/// The stages of started work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    InProgress,
    InReview,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Active {
    pub phase: Phase,
    pub whose: Whose,
    /// When the turn runs out. Only work from before turns had clocks lacks one.
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
        /// Why it's put off and what to do once it's back, said again when it wakes.
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
            Self::InProgress => Stage::InProgress,
            Self::InReview => Stage::InReview,
        }
    }

    pub const fn of(stage: Stage) -> Option<Self> {
        match stage {
            Stage::InProgress => Some(Self::InProgress),
            Stage::InReview => Some(Self::InReview),
            Stage::UpNext | Stage::Done => None,
        }
    }
}

impl Life {
    /// Active work at `phase` whose turn is `whose`, with no clock yet.
    pub const fn active_as(phase: Phase, whose: Whose, until: Option<DateTime<Utc>>) -> Self {
        Self::Active(Active {
            phase,
            whose,
            until,
        })
    }

    pub const fn stage(&self) -> Stage {
        match self {
            Self::Unstarted => Stage::UpNext,
            Self::Active(a) => a.phase.stage(),
            Self::Done => Stage::Done,
        }
    }

    pub const fn active(&self) -> Option<&Active> {
        match self {
            Self::Active(a) => Some(a),
            Self::Unstarted | Self::Done => None,
        }
    }

    pub const fn whose(&self) -> Option<&Whose> {
        match self {
            Self::Active(a) => Some(&a.whose),
            Self::Unstarted | Self::Done => None,
        }
    }

    pub const fn turn(&self) -> Option<Turn> {
        match self {
            Self::Active(a) => Some(a.whose.turn()),
            Self::Unstarted | Self::Done => None,
        }
    }

    pub fn until(&self) -> Option<DateTime<Utc>> {
        self.active().and_then(|a| a.until)
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

/// Started work missing a turn is the user's: the safe side, as it shows on Today.
fn whose(f: LifeFields) -> Whose {
    match f.turn.unwrap_or(Turn::You) {
        Turn::You => Whose::You {
            need: f.need.unwrap_or_default(),
        },
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
            return if f.stage == Stage::Done {
                Self::Done
            } else {
                Self::Unstarted
            };
        };
        let until = f.turn_until;
        Self::Active(Active {
            phase,
            whose: whose(f),
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
        if let Life::Active(a) = life {
            f.turn_until = a.until;
            whose_fields(a.whose, &mut f);
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
            Life::Unstarted,
            Life::Done,
            Life::Active(Active {
                phase: Phase::InReview,
                whose: Whose::You {
                    need: "Merge it?".to_owned(),
                },
                until: Some(Utc::now()),
            }),
            Life::Active(Active {
                phase: Phase::InProgress,
                whose: Whose::Others {
                    waiting_on: "CI".to_owned(),
                    then: Some(Turn::Agent),
                },
                until: None,
            }),
            Life::Active(Active {
                phase: Phase::InProgress,
                whose: Whose::Later {
                    then: Some(Turn::Agent),
                    wake: Vec::new(),
                    note: Some("traffic should be 0: remove the route".to_owned()),
                },
                until: Some(Utc::now()),
            }),
        ];
        for life in lives {
            assert_eq!(round_trip(life.clone()), life);
        }
    }

    #[test]
    fn unstarted_and_done_work_has_no_turn_whatever_was_stored() {
        let stored = |stage| LifeFields {
            stage,
            turn: Some(Turn::Agent),
            turn_until: Some(Utc::now()),
            need: None,
            waiting_on: None,
            then_turn: None,
            wake: Vec::new(),
        };
        assert_eq!(Life::from(stored(Stage::UpNext)).turn(), None);
        assert_eq!(Life::from(stored(Stage::Done)).turn(), None);
        let mut missing = stored(Stage::InProgress);
        missing.turn = None;
        assert_eq!(Life::from(missing).turn(), Some(Turn::You));
    }
}
