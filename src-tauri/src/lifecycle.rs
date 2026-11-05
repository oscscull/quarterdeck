//! How setting an item's stage moves it through its life. New or reopened work is the user's turn
//! (until someone takes it on), moving open work between stages keeps its turn and clock, and
//! finishing it ends the turn. No stage leaves open work without a turn.

use crate::domain::item::Stage;
use crate::domain::life::{Life, Open, Phase, Whose};

const STARTED: &str = "Started: what happens next?";
const REOPENED: &str = "Reopened: what happens next?";

/// The user's turn on newly open work, needing `label` if it says anything, else `fallback`.
fn yours(phase: Phase, label: &str, fallback: &str) -> Life {
    if phase == Phase::UpNext {
        return Life::not_started();
    }
    let need = if label.trim().is_empty() {
        fallback
    } else {
        label
    };
    let whose = Whose::You {
        need: need.trim().to_owned(),
    };
    Life::open_as(phase, whose, None)
}

/// The life of new work at `stage`.
pub fn new(stage: Stage, label: &str) -> Life {
    Phase::of(stage).map_or(Life::Done, |phase| yours(phase, label, STARTED))
}

/// `old` moved to `stage`. Open work keeps its turn, whatever stage it moves to.
pub fn moved(old: &Life, stage: Stage, label: &str) -> Life {
    match (old, Phase::of(stage)) {
        (_, None) => Life::Done,
        // Starting work nobody had decided on answers the question its need asked.
        (Life::Open(_), Some(phase)) if old.untouched() && phase != Phase::UpNext => {
            yours(phase, label, STARTED)
        }
        (Life::Open(o), Some(phase)) => Life::Open(Open { phase, ..o.clone() }),
        (Life::Done, Some(phase)) => yours(phase, label, REOPENED),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::turn::Turn;

    fn agents() -> Life {
        Life::open_as(Phase::InProgress, Whose::Agent, None)
    }

    #[test]
    fn open_work_keeps_its_turn_whatever_its_stage() {
        for stage in [Stage::UpNext, Stage::InProgress, Stage::InReview] {
            let moved = moved(&agents(), stage, "");
            assert_eq!((moved.stage(), moved.turn()), (stage, Some(Turn::Agent)));
        }
        assert_eq!(moved(&agents(), Stage::Done, ""), Life::Done);
        let started = moved(&Life::not_started(), Stage::InProgress, "");
        assert!(matches!(started.whose(), Some(Whose::You { need }) if need == STARTED));
    }

    #[test]
    fn new_or_reopened_work_is_the_users_turn_saying_what_it_needs() {
        assert_eq!(new(Stage::UpNext, "someday"), Life::not_started());
        assert_eq!(moved(&Life::Done, Stage::UpNext, ""), Life::not_started());
        let reopened = moved(&Life::Done, Stage::InReview, "review it");
        assert!(matches!(
            reopened.whose(),
            Some(Whose::You { need: wanted }) if wanted == "review it"
        ));
        let started = new(Stage::InProgress, "");
        assert!(matches!(
            started.whose(),
            Some(Whose::You { need: wanted }) if wanted == STARTED
        ));
    }
}
