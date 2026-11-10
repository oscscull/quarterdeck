//! While the agent can't run at all (offline, `claude` missing or logged out), its work waits
//! instead of failing: one banner says so, nothing is prompted, and Quarterdeck checks again,
//! waiting longer each time. The first good check clears it. Turn clocks keep running throughout,
//! so work on the agent's turn still comes back to the user if the outage outlasts it.

use chrono::{DateTime, Duration, Utc};
use tauri::{AppHandle, Manager as _};
use tokio::sync::Mutex;

use crate::app::App;
use crate::claude::reach;
use crate::config::settings::Settings;
use crate::domain::outage::Outage;
use crate::error::AppResult;
use crate::notify;
use crate::store::{Db, meta};

const SINCE: &str = "outage.since";
const NEXT: &str = "outage.next_check";
const GAP: &str = "outage.gap_seconds";
const WHY: &str = "outage.why";
const RECOVERED: &str = "outage.recovered";
const TITLE: &str = "Your agent can't run";
/// The longest reason the banner shows.
const WHY_CHARS: usize = 200;
/// Runs failing together report together; one at a time, they start one outage and wait once.
static REPORTING: Mutex<()> = Mutex::const_new(());

/// The outage going on, if the agent is down.
pub async fn current(db: &Db) -> AppResult<Option<Outage>> {
    let Some(since) = meta::time(db, SINCE).await? else {
        return Ok(None);
    };
    Ok(Some(Outage {
        why: meta::text(db, WHY).await?.unwrap_or_default(),
        since,
        next_check: meta::time(db, NEXT).await?,
    }))
}

/// Whether the agent can run now. While it's down this checks again only once the wait is over,
/// unless `now` is set.
pub async fn agent_up(handle: &AppHandle, settings: &Settings, now: bool) -> AppResult<bool> {
    let db = &handle.state::<App>().db;
    // A build that may not run the agent isn't down; its runs are turned away elsewhere.
    let Ok(command) = settings.claude() else {
        return Ok(true);
    };
    if !now && waiting(db).await? {
        return Ok(false);
    }
    match reach::check(command, &settings.outage).await {
        None => recover(db).await.map(|()| true),
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

/// Checks at once because someone asked, and says what it found: the outage still going on, or
/// `None` now the agent can run.
pub async fn check_now(db: &Db, settings: &Settings) -> AppResult<Option<Outage>> {
    match reach::check(settings.claude()?, &settings.outage).await {
        None => recover(db).await?,
        Some(why) => drop(note(db, settings, &why).await?),
    }
    current(db).await
}

/// The agent couldn't run: the first time starts the outage and tells the user, and each failed
/// check waits longer to check again.
pub async fn report(handle: &AppHandle, settings: &Settings, why: &str) -> AppResult<()> {
    let db = &handle.state::<App>().db;
    if note(db, settings, why).await? {
        // The banner shows it either way.
        drop(notify::send(handle, TITLE, &brief(why)));
    }
    Ok(())
}

async fn waiting(db: &Db) -> AppResult<bool> {
    let next = meta::time(db, NEXT).await?;
    Ok(next.is_some_and(|next| next > Utc::now()))
}

fn brief(why: &str) -> String {
    let line = why.lines().next().unwrap_or_default();
    line.chars().take(WHY_CHARS).collect()
}

fn seconds(n: u64) -> Duration {
    Duration::try_seconds(i64::try_from(n).unwrap_or(i64::MAX)).unwrap_or(Duration::MAX)
}

/// The wait before the last check, which the next one doubles. One left by an outage that ended a
/// while ago is forgotten; one that ended just now carries on, since a check can pass (the
/// network is up) while runs still can't get through (the API is overloaded).
async fn last_gap(db: &Db, settings: &Settings, now: DateTime<Utc>) -> AppResult<Option<u64>> {
    let settled = seconds(settings.outage.max_retry_seconds);
    if let Some(recovered) = meta::time(db, RECOVERED).await?
        && now.signed_duration_since(recovered) > settled
    {
        return Ok(None);
    }
    Ok(meta::text(db, GAP).await?.and_then(|g| g.parse().ok()))
}

/// Records that the agent couldn't run, and says whether that's news. Reports while already
/// waiting only update the reason.
async fn note(db: &Db, settings: &Settings, why: &str) -> AppResult<bool> {
    let _one = REPORTING.lock().await;
    let now = Utc::now();
    meta::set_text(db, WHY, &brief(why)).await?;
    let down = meta::time(db, SINCE).await?.is_some();
    if down && waiting(db).await? {
        return Ok(false);
    }
    let gap = last_gap(db, settings, now).await?;
    let next = settings.outage.next_gap(gap);
    meta::set_text(db, GAP, &next.to_string()).await?;
    meta::set_time(db, NEXT, now + seconds(next)).await?;
    meta::forget(db, RECOVERED).await?;
    if !down {
        meta::set_time(db, SINCE, now).await?;
    }
    Ok(gap.is_none())
}

/// Ends the outage, if there is one. Clocks aren't renewed: an outage that keeps coming back must
/// not keep work from ever reaching the user.
async fn recover(db: &Db) -> AppResult<()> {
    if meta::time(db, SINCE).await?.is_none() {
        return Ok(());
    }
    for key in [SINCE, NEXT, WHY] {
        meta::forget(db, key).await?;
    }
    meta::set_time(db, RECOVERED, Utc::now()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn failures_together_are_one_outage_until_it_recovers() {
        let db = Db::in_memory().await;
        let settings = Settings::default();
        assert!(note(&db, &settings, "Not logged in\nmore").await.unwrap());
        assert!(!note(&db, &settings, "offline").await.unwrap());
        let outage = current(&db).await.unwrap().unwrap();
        assert_eq!(outage.why, "offline");
        assert!(outage.next_check.is_some());

        recover(&db).await.unwrap();
        assert_eq!(current(&db).await.unwrap(), None);
    }

    #[tokio::test]
    async fn an_outage_that_comes_straight_back_is_not_news_and_waits_longer() {
        let db = Db::in_memory().await;
        let settings = Settings::default();
        note(&db, &settings, "overloaded").await.unwrap();
        recover(&db).await.unwrap();
        assert!(!note(&db, &settings, "overloaded").await.unwrap());
        let gap = meta::text(&db, GAP).await.unwrap();
        let doubled = settings.outage.first_retry_seconds * 2;
        assert_eq!(gap, Some(doubled.to_string()));
    }
}
