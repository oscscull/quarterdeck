//! Starts the schedules each new event's triggers match, on the machine that runs their area, and
//! wakes parked work waiting for it.

use chrono::{Duration, Utc};
use tauri::{AppHandle, Manager as _};

use super::tick::start;
use crate::app::App;
use crate::config::settings::Settings;
use crate::config::{pause, schedules};
use crate::domain::fired::{Fired, MAX_DEPTH};
use crate::domain::schedule::Schedule;
use crate::domain::trigger::OnEvent;
use crate::domain::urgent::Urgent;
use crate::error::AppResult;
use crate::store::{fired, runs, urgent};

/// Events taken per tick; any more wait for the next.
const BATCH: u32 = 100;
/// Runs looked through for the last one on a subject, for cooldowns.
const COOLDOWN_LOOKBACK: u32 = 20;

/// Handles every new event. While paused, events are marked handled and start nothing, though
/// they still wake parked work: that only hands the user back their turn.
pub async fn dispatch(handle: &AppHandle, settings: &Settings) -> AppResult<()> {
    let app = handle.state::<App>();
    let paused = pause::current(&app.config, Utc::now())?.is_some();
    let all = schedules::list(&app.config)?.schedules;
    for event in fired::new_events(&app.db, BATCH).await? {
        // One item failing to wake mustn't hold up every later event; its time still brings it back.
        drop(super::wake::on_event(&app.db, settings, &event).await);
        let matching = if paused {
            Vec::new()
        } else {
            matching(settings, &all, &event)
        };
        let mut started = Vec::new();
        for (schedule, trigger) in &matching {
            if may_start(handle, schedule, trigger, &event).await? {
                start(handle, (*schedule).clone(), Some(event.clone()));
                started.push(schedule.id.clone());
            }
        }
        fired::handled(&app.db, event.id, &started).await?;
        if !paused && super::ci::is_ci(&event) && settings.node_for(&event.area) == settings.node {
            // A failed hand-off mustn't stop the other events; the item still shows CI's state.
            drop(super::ci::react(handle, settings, &event, !matching.is_empty()).await);
        }
        if !paused
            && super::branch::is_branch(&event)
            && settings.node_for(&event.area) == settings.node
        {
            // Likewise: the item still shows the branch's state.
            drop(super::branch::react(handle, settings, &event, !matching.is_empty()).await);
        }
    }
    Ok(())
}

/// Enabled schedules run here, in the event's area, with a trigger the event matches.
fn matching<'a>(
    settings: &Settings,
    all: &'a [Schedule],
    event: &Fired,
) -> Vec<(&'a Schedule, &'a OnEvent)> {
    all.iter()
        .filter(|s| {
            s.enabled && s.area == event.area && settings.node_for(&s.area) == settings.node
        })
        .filter_map(|s| s.on_events().find(|t| event.matches(t)).map(|t| (s, t)))
        .collect()
}

/// Not while it's already running on the subject, inside its cooldown, or too far down a chain.
async fn may_start(
    handle: &AppHandle,
    schedule: &Schedule,
    trigger: &OnEvent,
    event: &Fired,
) -> AppResult<bool> {
    let db = &handle.state::<App>().db;
    if event.depth >= MAX_DEPTH {
        urgent::raise(db, &too_deep(schedule, event)).await?;
        return Ok(false);
    }
    let on_subject = |target: &str| target == event.subject;
    let live = runs::live(db).await?;
    if live
        .iter()
        .any(|r| r.schedule_id.as_deref() == Some(&schedule.id) && on_subject(&r.target))
    {
        return Ok(false);
    }
    if trigger.cooldown == 0 {
        return Ok(true);
    }
    let since = Utc::now() - Duration::minutes(i64::from(trigger.cooldown));
    let recent = runs::recent_for(db, &schedule.id, COOLDOWN_LOOKBACK).await?;
    Ok(!recent
        .iter()
        .any(|r| on_subject(&r.target) && r.started_at >= since))
}

fn too_deep(schedule: &Schedule, event: &Fired) -> Urgent {
    Urgent {
        id: 0,
        title: format!("{} didn't start", schedule.name),
        detail: format!(
            "{} came from {} events in a row, so schedules may be starting each other in a \
             loop. Check the triggers of the schedules involved.",
            event.kind, event.depth
        ),
        area: schedule.area.clone(),
        raised_at: Utc::now(),
        schedule_id: Some(schedule.id.clone()),
        run_id: None,
    }
}
