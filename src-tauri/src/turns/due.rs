//! What an item's clock calls for. Pure, so the rules are easy to test.

use chrono::{DateTime, Utc};

use crate::config::turns::TurnSettings;
use crate::domain::item::WorkItem;
use crate::domain::life::Whose;
use crate::domain::turn::Turn;

/// What Quarterdeck knows about the agent's work on an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Work {
    /// A run about the item is going.
    Running,
    /// An answer or message is waiting to go to the agent.
    Pending,
    /// A session outside Quarterdeck was heard from lately.
    Elsewhere,
    /// Nothing is happening.
    Idle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Due {
    Wait,
    /// In flight with no clock yet: start one.
    StartClock,
    /// The agent's turn and nothing is running: prompt it.
    Prompt,
    /// The user's turn ran out: remind them, and start the clock again.
    Remind,
    /// The turn ran out: pass it on, to the user or the agent.
    PassTo {
        to: Turn,
        why: String,
    },
}

/// Who a wait that ran out goes back to. Only the user or the agent, and the agent's own time runs
/// out to the user, so every wait ends with the user.
pub const fn back_to(then: Option<Turn>, default: Turn) -> Turn {
    match then {
        Some(Turn::Agent) => Turn::Agent,
        Some(Turn::You | Turn::Others | Turn::Later) => Turn::You,
        None => match default {
            Turn::Agent => Turn::Agent,
            Turn::You | Turn::Others | Turn::Later => Turn::You,
        },
    }
}

/// Why parked work came back: its note if it has one, which says what to do now.
pub fn back_from_later(note: Option<&str>) -> String {
    note.map_or_else(
        || "back from later: pick it up, put it off again, or close it".to_owned(),
        |n| format!("back from parking: {n}"),
    )
}

fn agent(work: Work, late: bool) -> Due {
    if late {
        let why = if work == Work::Running {
            "the agent's turn ran out while it was still running"
        } else {
            "the agent's turn ran out"
        };
        return Due::PassTo {
            to: Turn::You,
            why: why.to_owned(),
        };
    }
    match work {
        Work::Idle => Due::Prompt,
        Work::Running | Work::Pending | Work::Elsewhere => Due::Wait,
    }
}

/// What an active item's clock calls for. Unstarted and done work has no clock.
pub fn due(item: &WorkItem, work: Work, limits: &TurnSettings, now: DateTime<Utc>) -> Due {
    let Some(active) = item.life.active() else {
        return Due::Wait;
    };
    let Some(until) = active.until else {
        return Due::StartClock;
    };
    let late = now >= until;
    match &active.whose {
        Whose::Agent => agent(work, late),
        _ if !late => Due::Wait,
        Whose::You { .. } => Due::Remind,
        Whose::Others { waiting_on, then } => Due::PassTo {
            to: back_to(*then, limits.others_then),
            why: format!("waited {}h on {waiting_on}", limits.others_hours),
        },
        Whose::Later { then, note, .. } => Due::PassTo {
            to: back_to(*then, Turn::You),
            why: back_from_later(note.as_deref()),
        },
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;
    use crate::domain::life::{Life, Phase};

    fn item(whose: Whose, until: Option<i64>) -> WorkItem {
        let until = until.map(|m| Utc::now() + Duration::minutes(m));
        WorkItem::sample("a", Life::active_as(Phase::InReview, whose, until))
    }

    fn you() -> Whose {
        Whose::You {
            need: "Merge?".to_owned(),
        }
    }

    fn others(then: Option<Turn>) -> Whose {
        Whose::Others {
            waiting_on: "CI".to_owned(),
            then,
        }
    }

    fn check(i: &WorkItem, work: Work) -> Due {
        due(i, work, &TurnSettings::default(), Utc::now())
    }

    const IDLE: Work = Work::Idle;

    #[test]
    fn only_active_work_has_a_clock() {
        assert_eq!(check(&item(you(), None), IDLE), Due::StartClock);
        for life in [Life::Unstarted, Life::Done] {
            assert_eq!(check(&WorkItem::sample("a", life), IDLE), Due::Wait);
        }
    }

    #[test]
    fn the_agent_is_prompted_until_its_time_runs_out() {
        assert_eq!(check(&item(Whose::Agent, Some(60)), IDLE), Due::Prompt);
        let running = Work::Running;
        assert_eq!(check(&item(Whose::Agent, Some(60)), running), Due::Wait);
        assert_eq!(
            check(&item(Whose::Agent, Some(60)), Work::Elsewhere),
            Due::Wait
        );
        assert!(matches!(
            check(&item(Whose::Agent, Some(-1)), running),
            Due::PassTo { to: Turn::You, .. }
        ));
    }

    #[test]
    fn waits_end_and_the_user_is_reminded() {
        assert_eq!(check(&item(you(), Some(-1)), IDLE), Due::Remind);
        assert_eq!(check(&item(you(), Some(60)), IDLE), Due::Wait);
        assert_eq!(
            check(&item(others(None), Some(-1)), IDLE),
            Due::PassTo {
                to: Turn::You,
                why: "waited 24h on CI".to_owned()
            }
        );
        assert!(matches!(
            check(&item(others(Some(Turn::Agent)), Some(-1)), IDLE),
            Due::PassTo {
                to: Turn::Agent,
                ..
            }
        ));
    }

    #[test]
    fn a_wait_never_hands_on_to_another_wait() {
        for then in [Some(Turn::Others), Some(Turn::Later)] {
            assert!(matches!(
                check(&item(others(then), Some(-1)), IDLE),
                Due::PassTo { to: Turn::You, .. }
            ));
        }
        let limits = TurnSettings {
            others_then: Turn::Later,
            ..TurnSettings::default()
        };
        let late = item(others(None), Some(-1));
        assert!(matches!(
            due(&late, IDLE, &limits, Utc::now()),
            Due::PassTo { to: Turn::You, .. }
        ));
    }
}
