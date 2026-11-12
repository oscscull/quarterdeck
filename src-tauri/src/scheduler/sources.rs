//! Events the scheduler notices by itself each tick: meetings starting and ending, and watched
//! files changing.

use chrono::{DateTime, Duration, Utc};
use tauri::{AppHandle, Manager as _};

use crate::app::App;
use crate::config::schedules;
use crate::config::settings::Settings;
use crate::domain::commitment::Commitment;
use crate::domain::fired::{FILE_CHANGED, Fired};
use crate::domain::schedule::Schedule;
use crate::error::AppResult;
use crate::paths;
use crate::store::{Db, commitments, fired, meta};

/// Moments older than this when noticed (the machine slept) fire nothing, as with cron.
const GRACE: Duration = Duration::minutes(10);

/// Fires what happened between the last tick and `now`.
pub async fn fire(
    handle: &AppHandle,
    settings: &Settings,
    last: DateTime<Utc>,
    now: DateTime<Utc>,
) -> AppResult<()> {
    let app = handle.state::<App>();
    let lead = Duration::minutes(settings.alerts.lead_minutes);
    for meeting in commitments::list(&app.db).await? {
        meetings(&app.db, &meeting, lead, (last, now)).await?;
    }
    let all = schedules::list(&app.config)?.schedules;
    let here = all
        .iter()
        .filter(|s| s.enabled && settings.node_for(&s.area) == settings.node);
    for schedule in here {
        files(&app.db, schedule).await?;
    }
    Ok(())
}

/// True if `at` passed since the last tick, recently enough to still act on.
fn passed(at: DateTime<Utc>, (last, now): (DateTime<Utc>, DateTime<Utc>)) -> bool {
    at > last && at <= now && now - at <= GRACE
}

async fn meetings(
    db: &Db,
    m: &Commitment,
    lead: Duration,
    window: (DateTime<Utc>, DateTime<Utc>),
) -> AppResult<()> {
    let id = m.id.to_string();
    let moments = [
        ("meeting.starting", m.starts_at - lead, "starts soon"),
        ("meeting.ended", m.ends_at, "ended"),
    ];
    for (kind, at, what) in moments {
        if passed(at, window) {
            let about = format!("Meeting \"{}\" {what}", m.title);
            let event = Fired::new(kind, &m.area, &id, about)
                .with("meeting", &id)
                .with("title", &m.title);
            fired::emit(db, &event).await?;
        }
    }
    Ok(())
}

/// When a file was last changed, or for a folder its newest entry and how many there are.
fn stamp(path: &str) -> Option<String> {
    let full = paths::expand_home(path).ok()?;
    let meta = std::fs::metadata(&full).ok()?;
    let secs = |m: &std::fs::Metadata| {
        m.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_millis())
    };
    if !meta.is_dir() {
        return Some(format!("{}:{}", secs(&meta), meta.len()));
    }
    let entries: Vec<_> = std::fs::read_dir(&full).ok()?.flatten().collect();
    let newest = entries
        .iter()
        .filter_map(|e| e.metadata().ok())
        .map(|m| secs(&m))
        .max()
        .unwrap_or_default();
    Some(format!("{newest}:{}", entries.len()))
}

/// Fires `file.changed` for each path the schedule watches whose stamp moved since last seen.
/// The first look only records it.
async fn files(db: &Db, schedule: &Schedule) -> AppResult<()> {
    let paths = schedule
        .on_events()
        .filter(|t| t.on == FILE_CHANGED)
        .filter_map(|t| t.when.get("path"));
    for path in paths {
        let key = format!("file_seen.{}.{path}", schedule.area);
        let now = stamp(path).unwrap_or_else(|| "missing".to_owned());
        let before = meta::text(db, &key).await?;
        if before.as_deref() == Some(now.as_str()) {
            continue;
        }
        meta::set_text(db, &key, &now).await?;
        if before.is_some() {
            let event = Fired::new(
                FILE_CHANGED,
                &schedule.area,
                path,
                format!("{path} changed"),
            )
            .with("path", path);
            fired::emit(db, &event).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps_change_with_folder_contents() {
        let dir = std::env::temp_dir().join("quarterdeck-test-watch");
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.to_string_lossy().into_owned();
        let empty = stamp(&path).unwrap();
        std::fs::write(dir.join("new.txt"), "hi").unwrap();
        assert_ne!(stamp(&path).unwrap(), empty);
        assert!(stamp("/no/such/place").is_none());
    }
}
