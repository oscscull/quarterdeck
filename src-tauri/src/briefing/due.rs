//! Deciding when an area's briefing is rewritten. Deterministic: a fingerprint of what the briefing
//! is written from, checked each tick. A change that then holds still for `debounce_minutes` starts
//! one run, so a burst of changes costs one.

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use tauri::{AppHandle, Manager as _};

use super::inputs::{self, Inputs};
use crate::app::App;
use crate::config::settings::Settings;
use crate::error::AppResult;
use crate::store::{Db, meta};

const FINGERPRINT: &str = "briefing.fingerprint.";
/// Set while a rewrite is pending: when the area's work last changed.
const CHANGED: &str = "briefing.changed.";

/// FNV-1a: stable across builds, so an update doesn't look like a change.
fn fnv(text: &str) -> String {
    let hash = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{hash:016x}")
}

/// The parts of an item that should change the briefing. Turns and labels change too often.
#[derive(Serialize)]
struct ItemPrint<'a> {
    id: &'a str,
    title: &'a str,
    stage: crate::domain::item::Stage,
    parked: bool,
    project: Option<&'a str>,
    records: Vec<i64>,
}

/// What the briefing depends on, as a short hash.
pub fn fingerprint(inputs: &Inputs) -> AppResult<String> {
    let projects: Vec<_> = inputs
        .projects
        .iter()
        .map(|p| (&p.id, &p.title, &p.aim, p.state))
        .collect();
    let items: Vec<_> = inputs
        .items
        .iter()
        .map(|i| ItemPrint {
            id: &i.id,
            title: &i.title,
            stage: i.stage,
            parked: i.turn == Some(crate::domain::turn::Turn::Later),
            project: i.project.as_deref(),
            records: i.records.iter().map(|r| r.id).collect(),
        })
        .collect();
    let references: Vec<_> = inputs
        .references
        .iter()
        .map(|r| (r.id, &r.title, &r.quote, &r.project, &r.items))
        .collect();
    let text = serde_json::to_string(&(projects, items, references))?;
    Ok(fnv(&text))
}

/// Asks for `area`'s briefing to be rewritten on the next tick, e.g. after a correction.
pub async fn request(db: &Db, area: &str) -> AppResult<()> {
    meta::set_time(db, &format!("{CHANGED}{area}"), DateTime::<Utc>::UNIX_EPOCH).await
}

/// Whether `area` needs a rewrite now, noting any change since the last look.
async fn due(db: &Db, area: &str, print: &str, wait: Duration) -> AppResult<bool> {
    let now = Utc::now();
    let changed_key = format!("{CHANGED}{area}");
    let print_key = format!("{FINGERPRINT}{area}");
    if meta::text(db, &print_key).await?.as_deref() != Some(print) {
        meta::set_text(db, &print_key, print).await?;
        let pending = meta::time(db, &changed_key).await?;
        // A requested rewrite stays due; anything else waits for the work to settle.
        if pending != Some(DateTime::<Utc>::UNIX_EPOCH) {
            meta::set_time(db, &changed_key, now).await?;
        }
        return Ok(false);
    }
    let Some(changed) = meta::time(db, &changed_key).await? else {
        return Ok(false);
    };
    Ok(now - changed >= wait)
}

/// Starts a rewrite for each area run here whose work changed and has settled.
pub async fn check(handle: &AppHandle, settings: &Settings) -> AppResult<()> {
    let app = handle.state::<App>();
    let wait = Duration::minutes(settings.briefing.debounce_minutes);
    for area in &settings.areas {
        if settings.node_for(&area.id) != settings.node {
            continue;
        }
        let now = Utc::now();
        let found =
            inputs::gather(&app.db, &area.id, settings.briefing.just_done_days, now).await?;
        if found.is_empty() && found.current.is_none() {
            continue;
        }
        let print = fingerprint(&found)?;
        if due(&app.db, &area.id, &print, wait).await?
            && !super::run::running(&app.db, &area.id).await?
        {
            meta::forget(&app.db, &format!("{CHANGED}{}", area.id)).await?;
            super::run::start(handle, area.id.clone());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn waits_for_work_to_settle_then_runs_once() {
        let db = Db::in_memory().await;
        let wait = Duration::zero();
        assert!(
            !due(&db, "work", "a", wait).await.unwrap(),
            "first sight is a change"
        );
        assert!(due(&db, "work", "a", wait).await.unwrap(), "settled");
        assert!(!due(&db, "work", "b", wait).await.unwrap(), "changed again");
        meta::forget(&db, "briefing.changed.work").await.unwrap();
        assert!(
            !due(&db, "work", "b", wait).await.unwrap(),
            "nothing pending"
        );
    }

    #[tokio::test]
    async fn a_request_survives_a_change() {
        let db = Db::in_memory().await;
        let wait = Duration::minutes(10);
        due(&db, "work", "a", wait).await.unwrap();
        request(&db, "work").await.unwrap();
        assert!(!due(&db, "work", "b", wait).await.unwrap());
        assert!(due(&db, "work", "b", wait).await.unwrap());
    }
}
