//! Whose turn a PR is, by what GitHub says. GitHub never takes the turn from the agent.

use super::snapshot::Owned;
use crate::domain::life::Whose;
use crate::domain::turn::Turn;
use crate::error::AppResult;
use crate::store::{Db, items};
use crate::turns::{self, Clock, Pass};

/// What GitHub says the PR waits on, and whether Claude fixes that by itself (see
/// `snapshot::claude_fixes`).
#[derive(Debug, Clone, Copy)]
pub struct Waits<'a> {
    pub owned: &'a Owned,
    pub claude_fixes: bool,
}

/// The agent's turn when it waits on a fix Claude makes by itself; the user's when it waits on
/// them, or on a fix Claude doesn't make; else others', waiting on whoever GitHub says.
fn pass_for(waits: Waits<'_>) -> Pass {
    let label = waits.owned.label.clone();
    match waits.owned.holder.as_str() {
        "Claude" if waits.claude_fixes => Pass::to(Turn::Agent, label),
        "you" | "Claude" => Pass::to(Turn::You, label),
        who => Pass::to(Turn::Others, label).waiting_on(who),
    }
}

/// Hands the turn to whoever GitHub says has it, unless they have it already or the agent does.
/// A PR that ended has no turn to follow.
pub async fn follow(db: &Db, item_id: &str, waits: Waits<'_>, clock: Clock<'_>) -> AppResult<()> {
    let item = items::get(db, item_id).await?;
    let Some(active) = item.life.open() else {
        return Ok(());
    };
    let pass = pass_for(waits);
    if active.whose == Whose::Agent || active.whose == pass.whose() {
        return Ok(());
    }
    turns::pass(db, item_id, pass, clock).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::item::Stage;

    fn owned(holder: &str) -> Owned {
        Owned {
            title: "Fix".to_owned(),
            stage: Stage::InReview,
            status: None,
            label: "CI failing, needs a fix".to_owned(),
            holder: holder.to_owned(),
        }
    }

    #[test]
    fn a_fix_claude_makes_never_lands_on_the_user() {
        let claude = owned("Claude");
        let on = Waits {
            owned: &claude,
            claude_fixes: true,
        };
        assert_eq!(pass_for(on).whose(), Whose::Agent);
        let off = Waits {
            claude_fixes: false,
            ..on
        };
        assert!(matches!(pass_for(off).whose(), Whose::You { .. }));
    }
}
