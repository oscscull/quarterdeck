//! One run of `prompts/briefing.md` for an area, recorded like any schedule run.

use tauri::{AppHandle, Manager as _};

use crate::app::App;
use crate::claude::{self, reach};
use crate::config::prompts::{Prompt, render};
use crate::config::settings::Settings;
use crate::domain::run::Outcome;
use crate::domain::schedule::Schedule;
use crate::domain::script::Report;
use crate::error::AppResult;
use crate::jobs::{self, Begun, Ended};
use crate::scheduler::outage;
use crate::store::{Db, runs};

fn id(area: &str) -> String {
    format!("briefing-{area}")
}

/// The run as a schedule, so it shows and is recorded like one.
fn schedule(area: &str) -> Schedule {
    Schedule {
        id: id(area),
        name: "Briefing".to_owned(),
        prompt: String::new(),
        script: None,
        triggers: Vec::new(),
        area: area.to_owned(),
        enabled: true,
        request: String::new(),
        allowed_tools: Vec::new(),
    }
}

/// Whether `area`'s briefing is being written right now.
pub async fn running(db: &Db, area: &str) -> AppResult<bool> {
    let id = id(area);
    Ok(runs::live(db)
        .await?
        .iter()
        .any(|r| r.schedule_id.as_deref() == Some(id.as_str())))
}

/// Writes `area`'s briefing in the background.
pub fn start(handle: &AppHandle, area: String) {
    let handle = handle.clone();
    tauri::async_runtime::spawn(async move {
        drop(run(&handle, &area).await);
    });
}

async fn ask(app: &App, settings: &Settings, area: &str, begun: &Begun) -> AppResult<String> {
    let days = settings.briefing.just_done_days.to_string();
    let vars = [("area", area), ("just_done_days", days.as_str())];
    let prompt = render(&app.config, Prompt::Briefing, &vars)?;
    let opts = claude::Ask {
        run: Some(begun.id),
        depth: begun.depth,
        ..claude::Ask::default()
    };
    claude::ask_with(&settings.claude()?, &prompt, &opts).await
}

async fn run(handle: &AppHandle, area: &str) -> AppResult<()> {
    let app = handle.state::<App>();
    let settings = app.settings()?;
    let schedule = schedule(area);
    let begun = jobs::begin(&app, &schedule, None).await?;
    let (outcome, summary) = match ask(&app, &settings, area, &begun).await {
        Ok(text) => (Outcome::Ok, jobs::summary_of(&text)),
        Err(e) => (Outcome::Failed, jobs::summary_of(&e.to_string())),
    };
    // A run that never reached the agent is the outage's to report.
    let unreached = outcome == Outcome::Failed && reach::unreached(&summary);
    if unreached {
        outage::report(handle, &settings, &summary).await?;
    }
    let report = if unreached {
        Report::Never
    } else {
        Report::Failures
    };
    let ended = Ended {
        outcome,
        summary,
        report,
    };
    jobs::settle(handle, &schedule, &begun, &ended).await
}
