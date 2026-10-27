//! GitHub: the user's open PRs become items, and changes on them fire events.

mod adopt;
mod apply;
mod checks;
mod fate;
mod fetch;
pub mod issues;
pub mod link;
mod linked;
mod snapshot;
mod turn;
mod update;

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use tauri::{AppHandle, Manager as _};

use crate::app::App;
use crate::config::github::GithubSettings;
use crate::config::settings::Settings;
use crate::config::turns::TurnSettings;
use crate::connectors::status;
use crate::domain::fired::Fired;
use crate::domain::schedule::EventKind;
use crate::error::AppResult;
use crate::store::{Db, fired, items};

pub use adopt::{OpenedPr, adopt};
pub use apply::SOURCE;
pub use update::update_branch;

fn describe(kind: EventKind, ext: &str, url: &str) -> String {
    let what = match kind {
        EventKind::PrNewComments => "new review comments",
        EventKind::CiFailed => "CI failing",
        EventKind::CiPassed => "CI passed",
        EventKind::PrBehind => "behind its base branch",
        EventKind::PrConflicts => "conflicts with its base branch",
    };
    format!("{what} on {ext} ({url})")
}

/// The PR with the user as its reviewer when they review their own PRs in that repo.
fn as_reviewed(pr: &fetch::Pr, solo: bool) -> fetch::Pr {
    if solo {
        fetch::Pr {
            reviewers: fetch::Reviewers::Yourself,
            ..pr.clone()
        }
    } else {
        pr.clone()
    }
}

/// The PR's CI state without its informational checks, whose failures become notes.
fn with_checks(pr: &fetch::Pr, informational: &[String]) -> fetch::Pr {
    if informational.is_empty() {
        return pr.clone();
    }
    fetch::Pr {
        ci: checks::rollup(&pr.checks, informational).or_else(|| pr.ci.clone()),
        notes: checks::notes(&pr.checks, informational),
        ..pr.clone()
    }
}

/// Pulls open PRs into items, firing an event for each change worth reacting to.
pub async fn sync(handle: &AppHandle) -> AppResult<()> {
    let app = handle.state::<App>();
    let all = app.settings()?;
    let prs = fetch::open_prs().await?;
    // Before the PRs, so one an item already links to doesn't get a second item.
    linked::bind(&app.db, &all).await?;
    let now = Utc::now();
    let mut open = HashSet::new();
    for pr in prs.iter().filter(|p| !all.github.ignores(&p.repo)) {
        // Open on GitHub whichever machine syncs it, so it must never be closed here.
        open.insert(apply::external_id(pr));
        if let Some(area) = synced_here(&app.db, &all, pr).await? {
            let pr = prepared(pr, &all.github);
            sync_pr(&app.db, &pr, (&area, &all.turns), now).await?;
        }
    }
    let unseen = apply::unseen(&app.db, &open).await?;
    close_ended(&app.db, &unseen).await?;
    sync_issues(&app.db, &all, &prs).await
}

/// Syncs issues after PRs, recording how it went on its own, so a failure there never costs the
/// PRs' events.
async fn sync_issues(db: &Db, all: &Settings, prs: &[fetch::Pr]) -> AppResult<()> {
    let result = issues::sync(db, all, prs).await;
    let error = result.err().map(|e| e.to_string());
    status::record(db, issues::SOURCE, error.as_deref()).await
}

/// Asks GitHub what happened to each PR the search didn't return, and closes the merged or
/// closed ones. One GitHub can't answer for stays as it is until the next sync.
async fn close_ended(db: &Db, unseen: &[String]) -> AppResult<()> {
    for ext in unseen {
        if let Ok(ended) = fate::of(ext).await {
            apply::close(db, ext, ended, Utc::now()).await?;
        }
    }
    Ok(())
}

/// The area to sync `pr` into, if this machine syncs it: the area of the item already made for
/// it, else the repo's. Another machine syncs the areas it runs, so two never duplicate a PR.
async fn synced_here(db: &Db, all: &Settings, pr: &fetch::Pr) -> AppResult<Option<String>> {
    let ext = apply::external_id(pr);
    let area = items::by_external(db, SOURCE, &ext)
        .await?
        .map_or_else(|| all.github.area_for(&pr.repo).to_owned(), |i| i.area);
    Ok((all.node_for(&area) == all.node).then_some(area))
}

/// The PR as the user sees it: their own review where they self-review, CI without
/// informational checks.
fn prepared(pr: &fetch::Pr, settings: &GithubSettings) -> fetch::Pr {
    let pr = as_reviewed(pr, settings.self_reviews(&pr.repo));
    with_checks(&pr, &settings.informational(&pr.repo))
}

/// Brings one PR's item up to date and fires its events.
async fn sync_pr(
    db: &Db,
    pr: &fetch::Pr,
    (area, limits): (&str, &TurnSettings),
    now: DateTime<Utc>,
) -> AppResult<()> {
    let ext = apply::external_id(pr);
    let kinds = apply::apply(db, pr, (area, limits), now).await?;
    let item_id = items::by_external(db, SOURCE, &ext)
        .await?
        .map(|i| i.id)
        .unwrap_or_default();
    for kind in kinds {
        let event = Fired::new(kind.name(), area, &item_id, describe(kind, &ext, &pr.url))
            .with("item", &item_id)
            .with("pr", &ext)
            .with("url", &pr.url);
        fired::emit(db, &event).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::Area;

    fn area(id: &str, node: &str) -> Area {
        Area {
            id: id.to_owned(),
            name: id.to_owned(),
            color: String::new(),
            node: node.to_owned(),
        }
    }

    /// This machine runs `projects`; `work`, the repo's default area, runs elsewhere.
    fn settings() -> Settings {
        Settings {
            node: "home".to_owned(),
            areas: vec![area("work", "office"), area("projects", "home")],
            github: GithubSettings {
                default_area: "work".to_owned(),
                ..GithubSettings::default()
            },
            ..Settings::default()
        }
    }

    fn pr(number: u64) -> fetch::Pr {
        fetch::Pr {
            repo: "me/lib".to_owned(),
            number,
            title: "Fix".to_owned(),
            url: format!("https://github.com/me/lib/pull/{number}"),
            draft: false,
            review: None,
            ci: None,
            checks: Vec::new(),
            notes: Vec::new(),
            comments: 0,
            reviewers: fetch::Reviewers::Requested,
            reviewer_names: Vec::new(),
            closes: Vec::new(),
            base: fetch::Base::Current,
        }
    }

    #[tokio::test]
    async fn a_pr_syncs_where_its_item_is_held() {
        let db = Db::in_memory().await;
        // A PR in another machine's area with no item here is that machine's to sync.
        assert_eq!(synced_here(&db, &settings(), &pr(1)).await.unwrap(), None);
        // One that this machine already holds an item for (e.g. opened for a queued issue)
        // stays here, in that item's area.
        apply::apply(
            &db,
            &pr(2),
            ("projects", &TurnSettings::default()),
            Utc::now(),
        )
        .await
        .unwrap();
        assert_eq!(
            synced_here(&db, &settings(), &pr(2))
                .await
                .unwrap()
                .as_deref(),
            Some("projects")
        );
        // Seen in the open-PR search, so nothing asks GitHub whether it ended.
        let open = HashSet::from([apply::external_id(&pr(2))]);
        assert_eq!(
            apply::unseen(&db, &open).await.unwrap(),
            Vec::<String>::new()
        );
    }
}
