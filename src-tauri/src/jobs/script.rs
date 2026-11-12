//! Runs a script schedule: only once the user has approved it, in its folder, with what started it
//! in its environment.

use std::path::PathBuf;
use std::time::Duration;

use chrono::Utc;
use tauri::{AppHandle, Manager as _};

use super::approval;
use super::settle::{Begun, Ended, begin, settle, summary_of};
use super::shell::{self, Shell};
use crate::app::App;
use crate::domain::fired::Fired;
use crate::domain::run::Outcome;
use crate::domain::schedule::Schedule;
use crate::domain::script::Script;
use crate::domain::urgent::Urgent;
use crate::error::AppResult;
use crate::runlog::{self, Writer};
use crate::store::fired::DEPTH_VAR;
use crate::store::urgent;
use crate::{notify, paths};

/// No folder means the home folder.
fn folder(cwd: Option<&str>) -> AppResult<PathBuf> {
    paths::expand_home(cwd.unwrap_or("~"))
}

/// What the script can read about itself and why it's running. `QUARTERDECK` is the app, for
/// `"$QUARTERDECK" tool <name> '<json>'`; events it fires count as part of this run's chain.
fn environment(
    schedule: &Schedule,
    begun: &Begun,
    cause: Option<&Fired>,
) -> AppResult<Vec<(String, String)>> {
    let exe = std::env::current_exe()?.to_string_lossy().into_owned();
    let mut env = vec![
        ("QUARTERDECK", exe),
        ("QD_SCHEDULE", schedule.id.clone()),
        ("QD_AREA", schedule.area.clone()),
        ("QD_NODE", begun.node.clone()),
        ("QD_RUN_ID", begun.id.to_string()),
        (DEPTH_VAR, begun.depth.to_string()),
    ];
    if let Some(event) = cause {
        env.extend([
            ("QD_EVENT", event.kind.clone()),
            ("QD_EVENT_ABOUT", event.about.clone()),
            ("QD_SUBJECT", event.subject.clone()),
            ("QD_EVENT_FIELDS", serde_json::to_string(&event.data)?),
        ]);
    }
    Ok(env.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

async fn execute(
    app: &App,
    schedule: &Schedule,
    script: &Script,
    (begun, cause): (&Begun, Option<&Fired>),
) -> AppResult<shell::Exit> {
    app.settings()?.scripts()?;
    let shell = Shell {
        script: &schedule.prompt,
        cwd: folder(script.cwd.as_deref())?,
        env: environment(schedule, begun, cause)?,
        limit: Duration::from_secs(u64::from(script.timeout_minutes.max(1)) * 60),
    };
    let log = runlog::dir()
        .ok()
        .and_then(|dir| Writer::create(&dir, begun.id).ok());
    shell::run(&shell, log).await
}

const NEEDS_APPROVAL: &str = "needs your approval";

/// Tells the user once, rather than on every trigger, that the script is waiting for them.
async fn ask_approval(handle: &AppHandle, schedule: &Schedule) -> AppResult<()> {
    let db = &handle.state::<App>().db;
    let title = format!("{} {NEEDS_APPROVAL}", schedule.name);
    let open = urgent::list(db).await?;
    if open.iter().any(|u| u.title == title) {
        return Ok(());
    }
    let item = Urgent {
        id: 0,
        title,
        detail: "Its script was made or changed outside this window. Check it on the Agent tab \
                 and approve it to let it run."
            .to_owned(),
        area: schedule.area.clone(),
        raised_at: Utc::now(),
        schedule_id: Some(schedule.id.clone()),
        run_id: None,
    };
    urgent::raise(db, &item).await?;
    drop(notify::send(handle, &item.title, &item.detail));
    Ok(())
}

/// Runs the script if it's approved as it stands, and records how it went.
pub async fn run(handle: &AppHandle, schedule: &Schedule, cause: Option<&Fired>) -> AppResult<()> {
    let app = handle.state::<App>();
    let Some(script) = &schedule.script else {
        return Ok(());
    };
    if !approval::approved(&app.db, schedule).await? {
        return ask_approval(handle, schedule).await;
    }
    let begun = begin(&app, schedule, cause).await?;
    let (outcome, summary) = match execute(&app, schedule, script, (&begun, cause)).await {
        Ok(exit) if exit.ok => (Outcome::Ok, summary_of(&exit.line)),
        Ok(exit) => (Outcome::Failed, summary_of(&exit.line)),
        Err(e) => (Outcome::Failed, summary_of(&e.to_string())),
    };
    let ended = Ended {
        outcome,
        summary,
        report: script.report,
    };
    settle(handle, schedule, &begun, &ended).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_expand_home() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(folder(None).unwrap(), home);
        assert_eq!(folder(Some("~/code/app")).unwrap(), home.join("code/app"));
        assert_eq!(folder(Some("/tmp")).unwrap(), PathBuf::from("/tmp"));
    }
}
