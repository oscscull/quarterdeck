//! Work the agent left uncommitted in its worktree. Quarterdeck only publishes commits, so it would
//! never reach a PR, and the user would be left to dig it out. Whoever the agent handed the turn to,
//! it comes straight back to the agent to commit, up to `failure_retries` times in a row; then the
//! user gets it, told exactly where the changes are.

use chrono::{DateTime, Utc};

use crate::config::turns::TurnSettings;
use crate::domain::turn::Turn;
use crate::error::AppResult;
use crate::store::Db;
use crate::turns::{self, Clock, Pass};
use crate::workspace::{self, Workspace};

/// Starts the turn reason each time the agent is sent back to commit, so they can be counted.
const MARK: &str = "You left uncommitted changes in the worktree";
/// How many changed paths to name before "and n more".
const SHOWN: usize = 10;

/// Hands the turn back if the worktree has uncommitted changes. True when it did, so nothing is
/// published from a half-committed worktree.
pub async fn handle(
    db: &Db,
    (item_id, ws): (&str, &Workspace),
    limits: &TurnSettings,
    now: DateTime<Utc>,
) -> AppResult<bool> {
    let changed = workspace::uncommitted(ws).await?;
    if changed.is_empty() {
        return Ok(false);
    }
    let clock = Clock { limits, now };
    let list = listed(&changed);
    let tries = turns::history::streak(db, item_id, MARK, super::unpublished::published).await?;
    let budget = usize::try_from(limits.failure_retries).unwrap_or(usize::MAX);
    let pass = if tries < budget {
        let why = format!(
            "{MARK} ({list}). Quarterdeck only publishes commits: commit them on this branch, \
             or discard what shouldn't ship, then hand the turn on again"
        );
        Pass::to(Turn::Agent, why)
    } else {
        let dir = ws.dir.display();
        let why = format!("the agent kept leaving uncommitted changes ({list})");
        Pass::to(Turn::You, why).needing(format!("Commit or discard the changes in {dir}"))
    };
    turns::pass(db, item_id, pass, clock).await?;
    Ok(true)
}

/// `a.rs, b.rs and 3 more`.
fn listed(paths: &[String]) -> String {
    let shown = paths.iter().take(SHOWN).map(String::as_str);
    let names = shown.collect::<Vec<_>>().join(", ");
    match paths.len().saturating_sub(SHOWN) {
        0 => names,
        more => format!("{names} and {more} more"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_a_few_paths() {
        let few = ["a.rs".to_owned(), "b.rs".to_owned()];
        assert_eq!(listed(&few), "a.rs, b.rs");
        let many: Vec<String> = (0..12).map(|n| format!("{n}.rs")).collect();
        assert!(listed(&many).ends_with("9.rs and 2 more"));
    }
}
