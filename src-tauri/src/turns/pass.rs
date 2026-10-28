//! A turn changing hands: who gets it, why, and what their turn needs.

use chrono::{DateTime, Utc};

use crate::domain::life::Whose;
use crate::domain::trigger::OnEvent;
use crate::domain::turn::Turn;

/// Who the turn goes to, and why.
#[derive(Debug, Clone)]
pub struct Pass {
    pub to: Turn,
    /// What was done, or what ran out. Kept in the item's history.
    pub why: String,
    /// For the user's turn: what's needed from them. Unset, the reason stands in for it. For a later
    /// turn: what to do once it's back.
    pub need: Option<String>,
    /// For others' turn: who it waits on.
    pub waiting_on: Option<String>,
    /// When the turn runs out. Unset, it's the configured length from now, or, for an agent handing
    /// the turn to itself, the deadline it already has.
    pub until: Option<DateTime<Utc>>,
    /// For others' or a later turn: who gets it after.
    pub then: Option<Turn>,
    /// For a later turn: events that end it early.
    pub wake: Vec<OnEvent>,
}

impl Pass {
    pub fn to(to: Turn, why: impl Into<String>) -> Self {
        Self {
            to,
            why: why.into(),
            need: None,
            waiting_on: None,
            until: None,
            then: None,
            wake: Vec::new(),
        }
    }

    #[must_use]
    pub fn waiting_on(self, who: impl Into<String>) -> Self {
        Self {
            waiting_on: Some(who.into()),
            ..self
        }
    }

    #[must_use]
    pub fn needing(self, need: impl Into<String>) -> Self {
        Self {
            need: Some(need.into()),
            ..self
        }
    }

    /// The turn it hands over, carrying what that turn needs.
    pub fn whose(&self) -> Whose {
        let need = self
            .need
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty());
        match self.to {
            Turn::You => Whose::You {
                need: need.unwrap_or(&self.why).to_owned(),
            },
            Turn::Agent => Whose::Agent,
            Turn::Others => Whose::Others {
                waiting_on: self
                    .waiting_on
                    .clone()
                    .unwrap_or_else(|| "others".to_owned()),
                then: self.then,
            },
            Turn::Later => Whose::Later {
                then: self.then,
                wake: self.wake.clone(),
                note: need.map(str::to_owned),
            },
        }
    }
}
