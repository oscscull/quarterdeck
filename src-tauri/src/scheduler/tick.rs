//! One pass of the scheduler: take inbox files, then start or mark missed any due schedules, and
//! start the schedules new events trigger.

use chrono::{DateTime, Duration, Utc};
use tauri::{AppHandle, Manager as _};

use super::due::{Plan, plan};
use super::keeper::Keeping;
use crate::app::App;
use crate::config::settings::Settings;
use crate::config::{pause, schedules};
use crate::domain::fired::Fired;
use crate::domain::run::{Outcome, Run};
use crate::domain::schedule::Schedule;
use crate::domain::urgent::Urgent;
use crate::error::AppResult;
use crate::notify;
use crate::store::{meta, run_requests, runs, urgent};

const LAST_TICK: &str = "scheduler.last_tick";
/// A run this late still goes ahead; anything older counts as missed.
const GRACE: Duration = Duration::minutes(10);

/// The stretch of time since the previous tick.
#[derive(Debug, Clone, Copy)]
struct Window {
    last: DateTime<Utc>,
    now: DateTime<Utc>,
}

/// Clocks go first and run whatever else happens: a pause or a failing step later in the tick can
/// stop work starting, never a turn running out. Only an agent that can't run holds its own turns.
pub async fn tick(handle: &AppHandle) -> AppResult<()> {
    let app = handle.state::<App>();
    let settings = app.settings()?;
    let now = Utc::now();
    let up = super::outage::agent_up(handle, &settings, false)
        .await
        .unwrap_or(false);
    let paused = !matches!(pause::current(&app.config, now), Ok(None));
    if up {
        // Before the keeper, so what goes back to the agent is prompted this tick.
        drop(super::stranded::resume(&app.db, &settings.turns).await);
    }
    let keeping = Keeping {
        settings: &settings,
        prompting: up && !paused,
        agent_down: !up,
    };
    let kept = super::keeper::check(handle, keeping).await;
    let rest = the_rest(handle, &settings, Agent { up, paused }).await;
    kept.and(rest)
}

/// Whether the agent can run, and whether everything is paused.
#[derive(Clone, Copy)]
struct Agent {
    up: bool,
    paused: bool,
}

async fn the_rest(handle: &AppHandle, settings: &Settings, agent: Agent) -> AppResult<()> {
    let Agent { up, paused } = agent;
    let app = handle.state::<App>();
    let raised = app.take_inbox(settings).await?;
    if raised > 0 {
        // Notifications are a courtesy; a failure must not stop the scheduler.
        drop(notify::send(
            handle,
            "Quarterdeck",
            &format!("{raised} new urgent item(s)"),
        ));
    }
    super::peers::sync(handle);
    start_requested(handle).await?;
    crate::alerts::check(handle).await?;
    let urgent_count = urgent::list(&app.db).await?.len();
    crate::desktop::tray::show_urgent(handle, urgent_count)?;
    super::connectors::sync_if_due(handle, settings).await?;
    super::updates::check_if_due(handle, settings).await?;
    // Answers go out even while paused: the user just gave them.
    if up {
        super::answers::deliver(handle, settings).await?;
    }
    // Cleanup only; a failed removal is retried next tick.
    drop(crate::workspace::sweep(
        &crate::store::items::list(&app.db).await?,
    ));
    let now = Utc::now();
    let last = meta::time(&app.db, LAST_TICK).await?;
    meta::set_time(&app.db, LAST_TICK, now).await?;
    // First launch has no history to catch up on; a pause means nothing runs or is missed.
    let Some(last) = last else { return Ok(()) };
    super::sources::fire(handle, settings, last, now).await?;
    // Events wait while the agent is down, so their runs start once it's back. While paused they
    // are handled and start nothing, so a pause drops them rather than saving them up.
    if up {
        super::events::dispatch(handle, settings).await?;
    }
    if paused {
        return Ok(());
    }
    if up {
        super::queue::advance(handle, settings).await?;
    }
    start_due(handle, settings, Window { last, now }).await
}

/// Starts the schedules run here whose crons came due in `w`.
async fn start_due(handle: &AppHandle, settings: &Settings, w: Window) -> AppResult<()> {
    let app = handle.state::<App>();
    for schedule in schedules::list(&app.config)?.schedules {
        if schedule.enabled && settings.node_for(&schedule.area) == settings.node {
            consider(handle, settings, &schedule, w).await?;
        }
    }
    Ok(())
}

async fn consider(
    handle: &AppHandle,
    settings: &Settings,
    schedule: &Schedule,
    w: Window,
) -> AppResult<()> {
    let mut due = false;
    for expr in schedule.crons() {
        // A schedule with a bad expression is reported on the Agent tab instead.
        let Ok(p) = plan(expr, w.last, w.now, GRACE) else {
            continue;
        };
        if p.missed.is_some() {
            record_missed(handle, schedule, settings.node_for(&schedule.area), &p).await?;
        }
        due |= p.run;
    }
    if due && !is_running(handle, &schedule.id).await? {
        start(handle, schedule.clone(), None);
    }
    Ok(())
}

/// Starts schedules an agent asked to run now. These run even while paused: someone asked.
async fn start_requested(handle: &AppHandle) -> AppResult<()> {
    let app = handle.state::<App>();
    for id in run_requests::take_all(&app.db).await? {
        if let Ok(schedule) = schedules::find(&app.config, &id)
            && !is_running(handle, &id).await?
        {
            start(handle, schedule, None);
        }
    }
    Ok(())
}

async fn is_running(handle: &AppHandle, id: &str) -> AppResult<bool> {
    let live = runs::live(&handle.state::<App>().db).await?;
    Ok(live.iter().any(|r| r.schedule_id.as_deref() == Some(id)))
}

/// Runs a schedule in the background; the runner records its own outcome. `cause` is the event
/// that started it, if one did.
pub fn start(handle: &AppHandle, schedule: Schedule, cause: Option<Fired>) {
    let handle = handle.clone();
    tauri::async_runtime::spawn(async move {
        drop(crate::jobs::run(&handle, &schedule, cause.as_ref()).await);
    });
}

/// Records the latest missed occurrence as a run and raises it as urgent.
async fn record_missed(
    handle: &AppHandle,
    schedule: &Schedule,
    node: &str,
    p: &Plan,
) -> AppResult<()> {
    let Some(at) = p.missed else { return Ok(()) };
    let count = p.missed_count;
    let reason = "Quarterdeck wasn't running, or the machine was asleep".to_owned();
    let run = Run {
        id: 0,
        schedule_id: Some(schedule.id.clone()),
        what: schedule.name.clone(),
        target: String::new(),
        node: node.to_owned(),
        started_at: at,
        finished_at: Some(at),
        outcome: Some(Outcome::Missed),
        summary: Some(reason.clone()),
    };
    let times = if count > 1 {
        format!(" ({count} times)")
    } else {
        String::new()
    };
    let item = Urgent {
        id: 0,
        title: format!("{} didn't run{times}", schedule.name),
        detail: format!("Due on {node}. {reason}."),
        area: schedule.area.clone(),
        raised_at: Utc::now(),
        schedule_id: Some(schedule.id.clone()),
        run_id: None,
    };
    let db = &handle.state::<App>().db;
    runs::insert(db, &run).await?;
    urgent::raise(db, &item).await?;
    drop(notify::send(handle, &item.title, &item.detail));
    Ok(())
}
