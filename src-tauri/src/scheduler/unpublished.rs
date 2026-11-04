//! Work Quarterdeck couldn't push or open a PR for. Nothing is lost (the commits stay in the
//! worktree), so it's handed to whoever can fix it: with no connection, the agent, prompted once it's
//! back; otherwise the agent, to fix what stopped it (a rejected push, say), up to
//! `failure_retries` times in a row; then the user.

use chrono::{DateTime, Utc};
use tauri::{AppHandle, Manager as _};

use crate::app::App;
use crate::claude::reach;
use crate::domain::event::Change;
use crate::domain::turn::Turn;
use crate::domain::urgent::Urgent;
use crate::error::AppResult;
use crate::notify;
use crate::store::{Db, items, urgent};
use crate::turns::{self, Clock, Pass};
use crate::workspace::Workspace;

/// Starts the turn reason of every publish the agent is asked to fix, so they can be counted.
const MARK: &str = "Quarterdeck couldn't push the agent's work";
/// History notes that say the work did reach GitHub, which starts the count again.
const PUBLISHED: [&str; 2] = ["Quarterdeck pushed", "Quarterdeck opened"];

fn published(c: &Change) -> bool {
    matches!(c, Change::Note { text } if PUBLISHED.iter().any(|p| text.starts_with(p)))
}

pub async fn handle(
    app: &AppHandle,
    (item_id, ws): (&str, &Workspace),
    error: &str,
    now: DateTime<Utc>,
) -> AppResult<()> {
    let state = app.state::<App>();
    let settings = state.settings()?;
    let clock = Clock {
        limits: &settings.turns,
        now,
    };
    if reach::unreached(error) {
        let why = format!("{MARK} with no connection ({error}); it goes again once it's back");
        turns::pass(&state.db, item_id, Pass::to(Turn::Agent, why), clock).await?;
        return super::outage::report(app, &settings, error).await;
    }
    let tries = turns::history::streak(&state.db, item_id, MARK, published).await?;
    let budget = usize::try_from(settings.turns.failure_retries).unwrap_or(usize::MAX);
    if tries < budget {
        let why = format!("{MARK} or open its PR: {error}. Fix what stopped it");
        return turns::pass(&state.db, item_id, Pass::to(Turn::Agent, why), clock).await;
    }
    to_user(&state.db, (item_id, ws), error, clock).await?;
    // The urgent item carries it; the notification is a courtesy.
    let title = format!("Couldn't publish Claude's work on {item_id}");
    drop(notify::send(app, &title, error));
    Ok(())
}

async fn to_user(
    db: &Db,
    (item_id, ws): (&str, &Workspace),
    error: &str,
    clock: Clock<'_>,
) -> AppResult<()> {
    let now = clock.now;
    let why = format!("Quarterdeck couldn't push the work or open its PR: {error}");
    turns::pass(db, item_id, Pass::to(Turn::You, why), clock).await?;
    let raised = Urgent {
        id: 0,
        title: format!("Couldn't publish Claude's work on {item_id}"),
        detail: format!("{error}. The commits are in {}.", ws.dir.display()),
        area: items::get(db, item_id).await?.area,
        raised_at: now,
        schedule_id: None,
        run_id: None,
    };
    urgent::raise(db, &raised).await.map(drop)
}
