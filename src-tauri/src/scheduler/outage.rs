//! While the agent can't run at all (offline, `claude` missing), its work waits instead of failing:
//! one card says so, nothing is prompted, and Quarterdeck checks again, waiting longer each time.
//! The first good check clears the card. Turn clocks keep running throughout, so work on the
//! agent's turn still comes back to the user if the outage outlasts it.

use chrono::{Duration, Utc};
use tauri::{AppHandle, Manager as _};
use tokio::sync::Mutex;

use crate::app::App;
use crate::claude::reach;
use crate::config::settings::Settings;
use crate::domain::urgent::Urgent;
use crate::error::AppResult;
use crate::notify;
use crate::store::{Db, meta, urgent};

const SINCE: &str = "outage.since";
const NEXT: &str = "outage.next_check";
const GAP: &str = "outage.gap_seconds";
const CARD: &str = "outage.card";
const TITLE: &str = "Your agent can't run";
/// Runs failing together report together; one at a time, they raise one card and wait once.
static REPORTING: Mutex<()> = Mutex::const_new(());

/// Whether the agent can run now. While it's down this checks again only once the wait is over,
/// unless `now` is set.
pub async fn agent_up(handle: &AppHandle, settings: &Settings, now: bool) -> AppResult<bool> {
    let db = &handle.state::<App>().db;
    // A build that may not run the agent isn't down; its runs are turned away elsewhere.
    let Ok(command) = settings.claude() else {
        return Ok(true);
    };
    let down = meta::time(db, SINCE).await?.is_some();
    if down
        && !now
        && meta::time(db, NEXT)
            .await?
            .is_some_and(|next| next > Utc::now())
    {
        return Ok(false);
    }
    match reach::check(command, &settings.outage).await {
        None if down => recover(db).await.map(|()| true),
        None => Ok(true),
        Some(why) => report(handle, settings, &why).await.map(|()| false),
    }
}

/// Checks at once if the agent is down, e.g. when the user comes back to the window.
pub async fn recheck(handle: &AppHandle) -> AppResult<()> {
    let app = handle.state::<App>();
    if meta::time(&app.db, SINCE).await?.is_some() {
        agent_up(handle, &app.settings()?, true).await?;
    }
    Ok(())
}

/// The agent couldn't run: the first time raises the card, and each failed check waits longer to
/// check again. Reports while already waiting change nothing.
pub async fn report(handle: &AppHandle, settings: &Settings, why: &str) -> AppResult<()> {
    let _one = REPORTING.lock().await;
    let db = &handle.state::<App>().db;
    if meta::time(db, NEXT)
        .await?
        .is_some_and(|next| next > Utc::now())
    {
        return Ok(());
    }
    let gap = meta::text(db, GAP).await?.and_then(|g| g.parse().ok());
    let next = settings.outage.next_gap(gap);
    meta::set_text(db, GAP, &next.to_string()).await?;
    let wait = Duration::seconds(i64::try_from(next).unwrap_or(i64::MAX));
    meta::set_time(db, NEXT, Utc::now() + wait).await?;
    if gap.is_none() {
        meta::set_time(db, SINCE, Utc::now()).await?;
        raise_card(db, settings, why).await?;
        // The card shows it either way.
        drop(notify::send(handle, TITLE, why));
    }
    Ok(())
}

async fn raise_card(db: &Db, settings: &Settings, why: &str) -> AppResult<()> {
    let card = Urgent {
        id: 0,
        title: TITLE.to_owned(),
        detail: format!(
            "{why}. Its work waits on the agent's turn; Quarterdeck checks again by itself, and this \
             clears when the agent is back."
        ),
        area: settings.known_area(""),
        raised_at: Utc::now(),
        schedule_id: None,
        run_id: None,
    };
    let id = urgent::raise(db, &card).await?;
    meta::set_text(db, CARD, &id.to_string()).await
}

/// Clears the card. Clocks aren't renewed: an outage that keeps coming back must not keep work
/// from ever reaching the user.
async fn recover(db: &Db) -> AppResult<()> {
    if let Some(id) = meta::text(db, CARD).await?.and_then(|c| c.parse().ok()) {
        urgent::dismiss(db, id).await?;
    }
    for key in [SINCE, NEXT, GAP, CARD] {
        meta::forget(db, key).await?;
    }
    Ok(())
}
