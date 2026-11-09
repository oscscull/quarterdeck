//! After Claude finishes in a worktree, Quarterdeck does the GitHub side: it pushes the commits and,
//! for an issue, opens the PR and makes the item that PR's. Nothing is published while a question
//! on the item is open, since the work is waiting on the user, nor while the worktree has changes no
//! commit holds (see `uncommitted`). When it can't, see `unpublished`.

use chrono::{DateTime, Utc};
use tauri::{AppHandle, Manager as _};

use crate::app::App;
use crate::config::turns::TurnSettings;
use crate::connectors::github::{self, OpenedPr};
use crate::domain::event::Change;
use crate::error::AppResult;
use crate::notify;
use crate::store::{Db, asks, events};
use crate::turns::Clock;
use crate::workspace::{self, Published, Workspace};

pub async fn after_run(
    handle: &AppHandle,
    (item_id, ws): (&str, &Workspace),
    limits: &TurnSettings,
) {
    let app = handle.state::<App>();
    let now = Utc::now();
    match super::uncommitted::handle(&app.db, (item_id, ws), limits, now).await {
        Ok(false) => {}
        Ok(true) => return,
        Err(problem) => {
            // The commits can still go out; the notification is a courtesy.
            let title = format!("Couldn't check Claude's worktree on {item_id}");
            drop(notify::send(handle, &title, &problem.to_string()));
        }
    }
    let waiting = asks::for_item(&app.db, item_id)
        .await
        .map_or(true, |open| !open.is_empty());
    if waiting {
        return;
    }
    let recorded = match workspace::publish(ws).await {
        Ok(published) => record(&app.db, (item_id, ws, limits), published, now).await,
        Err(e) => super::unpublished::handle(handle, (item_id, ws), &e.to_string(), now).await,
    };
    if let Err(problem) = recorded {
        // Nothing else is left to tell; the notification is a courtesy.
        let title = format!("Couldn't publish Claude's work on {item_id}");
        drop(notify::send(handle, &title, &problem.to_string()));
    }
}

async fn record(
    db: &Db,
    (item_id, ws, limits): (&str, &Workspace, &TurnSettings),
    published: Published,
    now: DateTime<Utc>,
) -> AppResult<()> {
    match published {
        Published::Nothing => Ok(()),
        Published::Pushed => {
            let text = "Quarterdeck pushed Claude's commits".to_owned();
            events::record(db, item_id, now, &Change::Note { text }).await
        }
        Published::Pr { number, url } => {
            let pr = OpenedPr {
                repo: &ws.repo,
                number,
                url: &url,
            };
            github::adopt(db, item_id, pr, Clock { limits, now }).await
        }
    }
}
