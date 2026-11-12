//! Records a schedule run's start and end: run history, the `run.finished` event, and what the
//! user hears about it.

use chrono::Utc;
use tauri::{AppHandle, Manager as _};

use crate::app::App;
use crate::domain::activity::Activity;
use crate::domain::fired::Fired;
use crate::domain::run::{Outcome, Run};
use crate::domain::schedule::Schedule;
use crate::domain::script::Report;
use crate::domain::urgent::Urgent;
use crate::error::AppResult;
use crate::notify;
use crate::store::{activity, fired, runs, urgent};

const SUMMARY_CHARS: usize = 200;

/// The first real line, kept short: run history is for glancing, not reading.
pub fn summary_of(text: &str) -> String {
    text.lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or_default()
        .chars()
        .take(SUMMARY_CHARS)
        .collect()
}

/// A started run: its id, the machine it's on, and how many events led to it, counting itself.
#[derive(Debug, Clone)]
pub struct Begun {
    pub id: i64,
    pub node: String,
    pub depth: u32,
}

/// Records that `schedule` started, against the event's subject if an event started it.
pub async fn begin(app: &App, schedule: &Schedule, cause: Option<&Fired>) -> AppResult<Begun> {
    let node = app.settings()?.node_for(&schedule.area).to_owned();
    let run = Run {
        id: 0,
        schedule_id: Some(schedule.id.clone()),
        what: schedule.name.clone(),
        target: cause.map(|c| c.subject.clone()).unwrap_or_default(),
        node: node.clone(),
        started_at: Utc::now(),
        finished_at: None,
        outcome: None,
        summary: None,
    };
    let id = runs::start(&app.db, &run).await?;
    let depth = cause.map_or(0, |c| c.depth) + 1;
    Ok(Begun { id, node, depth })
}

/// How a run ended.
#[derive(Debug, Clone)]
pub struct Ended {
    pub outcome: Outcome,
    pub summary: String,
    pub report: Report,
}

fn finished(schedule: &Schedule, ended: &Ended, depth: u32) -> Fired {
    let word = match ended.outcome {
        Outcome::Ok => "ok",
        Outcome::Failed | Outcome::Missed => "failed",
    };
    let about = format!("{} finished ({word}): {}", schedule.name, ended.summary);
    Fired::new("run.finished", &schedule.area, &schedule.id, about)
        .with("schedule", &schedule.id)
        .with("outcome", word)
        .at_depth(depth)
}

fn note(schedule: &Schedule, summary: &str) -> Activity {
    Activity {
        id: 0,
        text: format!("{}: {summary}", schedule.name),
        area: schedule.area.clone(),
        at: Utc::now(),
        unread: true,
    }
}

fn failure(schedule: &Schedule, detail: String) -> Urgent {
    Urgent {
        id: 0,
        title: format!("{} failed", schedule.name),
        detail,
        area: schedule.area.clone(),
        raised_at: Utc::now(),
        schedule_id: Some(schedule.id.clone()),
        run_id: None,
    }
}

/// Finishes the run and tells the user as `report` says: success lands in "From Claude",
/// failure is raised as urgent, each with a quiet notification.
pub async fn settle(
    handle: &AppHandle,
    schedule: &Schedule,
    begun: &Begun,
    ended: &Ended,
) -> AppResult<()> {
    let app = handle.state::<App>();
    runs::finish(&app.db, begun.id, ended.outcome, &ended.summary).await?;
    fired::emit(&app.db, &finished(schedule, ended, begun.depth)).await?;
    let ok = ended.outcome == Outcome::Ok;
    let title = match (ok, ended.report) {
        (true, Report::Always) => {
            activity::insert(&app.db, &note(schedule, &ended.summary)).await?;
            schedule.name.clone()
        }
        (false, Report::Always | Report::Failures) => {
            let item = failure(schedule, ended.summary.clone());
            urgent::raise(&app.db, &item).await?;
            item.title
        }
        (true, Report::Failures | Report::Never) | (false, Report::Never) => return Ok(()),
    };
    // Notifications are a courtesy; the outcome is already recorded.
    drop(notify::send(handle, &title, &ended.summary));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_is_the_first_real_line() {
        assert_eq!(
            summary_of("\n  \nDid the thing.\nMore detail."),
            "Did the thing."
        );
        assert_eq!(summary_of(&"x".repeat(500)).len(), SUMMARY_CHARS);
    }
}
