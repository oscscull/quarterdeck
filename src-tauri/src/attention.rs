//! Where each item stands with the user, decided in one place so Today and the turn clock can't
//! disagree. Every open item is someone's turn and finishing it (or deleting it) is the only way
//! off Today for good: the user's turns, questions and issues to triage are on Today, and anyone
//! else's turn has a clock no longer than a day that brings it back (see `turns::due`). There is
//! no backlog: work nobody has started is the user's turn to start, park or close.

use crate::connectors::github::issues;
use crate::domain::item::WorkItem;
use crate::domain::life::{Life, Whose};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attention {
    /// The user's turn, or someone asked them something: on Today.
    Today,
    /// An issue nobody has looked at: on Today, counted on its triage line.
    Triage,
    /// The agent's, others' or a later turn: its clock brings it back.
    Returning,
    Closed,
}

/// Where `item` stands, `asked` being whether a question on it is open. A question is on Today
/// even on done work, so it can't be lost.
pub fn of(item: &WorkItem, asked: bool) -> Attention {
    if asked {
        return Attention::Today;
    }
    match &item.life {
        Life::Done => Attention::Closed,
        Life::Open(_) if issues::untriaged(item) => Attention::Triage,
        Life::Open(o) => match o.whose {
            Whose::You { .. } => Attention::Today,
            Whose::Agent | Whose::Others { .. } | Whose::Later { .. } => Attention::Returning,
        },
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::*;
    use crate::config::turns::{LONGEST_CHECK_HOURS, TurnSettings};
    use crate::domain::life::Phase;
    use crate::domain::trigger::OnEvent;
    use crate::domain::turn::Turn;
    use crate::turns::due::{Due, Work, due};
    use crate::turns::{Clock, Pass};

    const THENS: [Option<Turn>; 5] = [
        None,
        Some(Turn::You),
        Some(Turn::Agent),
        Some(Turn::Others),
        Some(Turn::Later),
    ];

    fn every_whose() -> Vec<Whose> {
        let wake = vec![OnEvent {
            on: "item.stage".to_owned(),
            when: std::collections::BTreeMap::new(),
            cooldown: 0,
        }];
        let mut all = vec![
            Whose::You {
                need: "Merge?".to_owned(),
            },
            Whose::Agent,
        ];
        for then in THENS {
            all.push(Whose::Others {
                waiting_on: "CI".to_owned(),
                then,
            });
            all.push(Whose::Later {
                then,
                wake: Vec::new(),
                note: None,
            });
            all.push(Whose::Later {
                then,
                wake: wake.clone(),
                note: Some("check traffic".to_owned()),
            });
        }
        all
    }

    fn every_life() -> Vec<Life> {
        let untils =
            [None, Some(-60), Some(60)].map(|m| m.map(|m| Utc::now() + Duration::minutes(m)));
        let phases = [Phase::UpNext, Phase::InProgress, Phase::InReview];
        let active = phases.into_iter().flat_map(|phase| {
            every_whose().into_iter().flat_map(move |whose| {
                untils.map(|until| Life::open_as(phase, whose.clone(), until))
            })
        });
        std::iter::once(Life::Done).chain(active).collect()
    }

    /// Lets time run on `item` with an agent that never hands over, returning where it settles.
    fn settle(mut item: WorkItem, asked: bool, limits: &TurnSettings) -> Attention {
        let mut now = Utc::now();
        for _ in 0..8 {
            let place = of(&item, asked);
            if place != Attention::Returning {
                return place;
            }
            let Some(active) = item.life.open() else {
                return place;
            };
            let deadline = active.until.unwrap_or(now);
            assert!(
                deadline <= now + Duration::hours(LONGEST_CHECK_HOURS),
                "{:?} waits over a day unchecked",
                item.life
            );
            now = deadline.max(now) + Duration::seconds(1);
            let (whose, phase) = match due(&item, Work::Running, limits, now) {
                Due::StartClock | Due::Remind | Due::Prompt => (active.whose.clone(), active.phase),
                Due::PassTo { to, why } => (Pass::to(to, why).whose(), active.phase),
                Due::Wait => panic!("{:?} waits forever", item.life),
            };
            let until = Clock { limits, now }.deadline(whose.turn()).unwrap();
            item.life = Life::open_as(phase, whose, Some(until));
        }
        panic!("{:?} never came back", item.life)
    }

    /// Where every item must end up: on Today unless it's done and nobody asked anything.
    fn expected(life: &Life, asked: bool) -> Attention {
        match (life, asked) {
            (Life::Done, false) => Attention::Closed,
            _ => Attention::Today,
        }
    }

    #[test]
    fn nothing_open_is_lost_whatever_its_state() {
        let thens = [Turn::You, Turn::Agent, Turn::Others, Turn::Later];
        let cases = thens.into_iter().flat_map(|others_then| {
            every_life()
                .into_iter()
                .flat_map(move |life| [false, true].map(|asked| (others_then, life.clone(), asked)))
        });
        for (others_then, life, asked) in cases {
            let limits = TurnSettings {
                others_then,
                ..TurnSettings::default()
            };
            let end = settle(WorkItem::sample("a", life.clone()), asked, &limits);
            assert_eq!(end, expected(&life, asked), "{life:?} asked: {asked}");
        }
    }
}
