//! Whose turn a PR is, by what GitHub says. GitHub never takes the turn from the agent.

use super::snapshot::Owned;
use crate::domain::life::Whose;
use crate::domain::turn::Turn;
use crate::error::AppResult;
use crate::store::{Db, items};
use crate::turns::{self, Clock, Pass};

/// The user's turn when it waits on them (or on a CI fix nobody started), else others', waiting
/// on whoever GitHub says: reviewers or CI.
fn pass_for(owned: &Owned) -> Pass {
    match owned.holder.as_str() {
        "you" | "Claude" => Pass::to(Turn::You, owned.label.clone()),
        who => Pass::to(Turn::Others, owned.label.clone()).waiting_on(who),
    }
}

/// Hands the turn to whoever GitHub says has it, unless they have it already or the agent does.
/// A PR that ended has no turn to follow.
pub async fn follow(db: &Db, item_id: &str, owned: &Owned, clock: Clock<'_>) -> AppResult<()> {
    let item = items::get(db, item_id).await?;
    let Some(active) = item.life.open() else {
        return Ok(());
    };
    let pass = pass_for(owned);
    if active.whose == Whose::Agent || active.whose == pass.whose() {
        return Ok(());
    }
    turns::pass(db, item_id, pass, clock).await
}
