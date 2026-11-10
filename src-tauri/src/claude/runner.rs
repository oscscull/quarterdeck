//! Runs one schedule through headless Claude.

use tauri::{AppHandle, Manager as _};

use super::reach;
use crate::app::App;
use crate::config::prompts::{Prompt, render};
use crate::domain::fired::Fired;
use crate::domain::run::Outcome;
use crate::domain::schedule::Schedule;
use crate::domain::script::Report;
use crate::error::AppResult;
use crate::jobs::{self, Begun, Ended};
use crate::scheduler::outage;

/// Why it's running, as the prompt says it: the event's line and its fields.
fn why(cause: Option<&Fired>) -> AppResult<String> {
    let Some(event) = cause else {
        return Ok("its schedule came due, or someone asked to run it now".to_owned());
    };
    let fields = serde_json::to_string(&event.data)?;
    Ok(format!(
        "{} (event {}, fields {fields})",
        event.about, event.kind
    ))
}

async fn ask(app: &App, schedule: &Schedule, begun: &Begun, trigger: &str) -> AppResult<String> {
    let settings = app.settings()?;
    let vars = [
        ("node", begun.node.as_str()),
        ("name", schedule.name.as_str()),
        ("instructions", schedule.prompt.as_str()),
        ("trigger", trigger),
    ];
    let prompt = render(&app.config, Prompt::RunSchedule, &vars)?;
    let opts = super::Ask {
        tools: &schedule.allowed_tools,
        session: super::Session::Fresh,
        cwd: None,
        permission_mode: None,
        run: Some(begun.id),
        depth: begun.depth,
    };
    super::ask_with(settings.claude()?, &prompt, &opts).await
}

/// Runs `schedule` now and records how it went. `cause` is the event that started it, if one
/// did, e.g. new comments on a PR; what the run does counts as one event further down its chain.
pub async fn run(handle: &AppHandle, schedule: &Schedule, cause: Option<&Fired>) -> AppResult<()> {
    let app = handle.state::<App>();
    let begun = jobs::begin(&app, schedule, cause).await?;
    let (outcome, summary) = match ask(&app, schedule, &begun, &why(cause)?).await {
        Ok(text) => (Outcome::Ok, jobs::summary_of(&text)),
        Err(e) => (Outcome::Failed, jobs::summary_of(&e.to_string())),
    };
    // A run that never reached the agent is the outage's to report, once for all of them.
    let unreached = outcome == Outcome::Failed && reach::unreached(&summary);
    if unreached {
        outage::report(handle, &app.settings()?, &summary).await?;
    }
    let report = if unreached {
        Report::Never
    } else {
        Report::Always
    };
    let ended = Ended {
        outcome,
        summary,
        report,
    };
    jobs::settle(handle, schedule, &begun, &ended).await
}
