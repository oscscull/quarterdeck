//! GitHub issues: open issues of watched owners and repos become items to triage, with no agent.

mod apply;
mod by_hand;
mod fate;
mod fetch;

use std::collections::{HashMap, HashSet};

use chrono::Utc;

pub use apply::{SOURCE, untriaged};

use super::apply::{SOURCE as PR_SOURCE, external_id as pr_external_id};
use super::fetch::Pr;
use crate::config::settings::Settings;
use crate::domain::item::Stage;
use crate::error::AppResult;
use crate::store::{Db, items};
use fetch::Issue;

/// Each issue one of the user's open PRs closes, with that PR. Only a PR with an open, visible
/// item here stands for an issue: one ignored, synced elsewhere, deleted or marked done here would
/// leave the issue with nothing standing for it.
async fn covering(db: &Db, prs: &[Pr]) -> AppResult<HashMap<String, String>> {
    let tracked: HashSet<String> = items::list(db)
        .await?
        .into_iter()
        .filter(|i| i.source == PR_SOURCE && i.stage() != Stage::Done)
        .filter_map(|i| i.external_id)
        .collect();
    Ok(prs
        .iter()
        .filter(|pr| tracked.contains(&pr_external_id(pr)))
        .flat_map(|pr| pr.closes.iter().map(|i| (i.clone(), pr_external_id(pr))))
        .collect())
}

fn wanted(all: &Settings, issue: &Issue) -> bool {
    let github = &all.github;
    github.issues.watches(&issue.repo)
        && !github.ignores(&issue.repo)
        && !github.issues.skips(&issue.labels)
}

/// The area to sync `issue` into, if this machine syncs it: its item's, else the repo's.
async fn synced_here(db: &Db, all: &Settings, issue: &Issue) -> AppResult<Option<String>> {
    let area = items::by_external(db, SOURCE, &issue.external_id())
        .await?
        .map_or_else(|| all.github.area_for(&issue.repo).to_owned(), |i| i.area);
    Ok((all.node_for(&area) == all.node).then_some(area))
}

/// Brings watched repos' open issues into items. `prs` are the user's open PRs, which stand for
/// the issues they close.
pub async fn sync(db: &Db, all: &Settings, prs: &[Pr]) -> AppResult<()> {
    if all.github.issues.watch.is_empty() {
        return Ok(());
    }
    let issues = fetch::open_issues(&all.github.issues.watch).await?;
    let covered = covering(db, prs).await?;
    let now = Utc::now();
    let mut open = HashSet::new();
    for issue in issues.iter().filter(|i| wanted(all, i)) {
        let ext = issue.external_id();
        open.insert(ext.clone());
        let Some(area) = synced_here(db, all, issue).await? else {
            continue;
        };
        match covered.get(&ext) {
            Some(pr) => apply::cover(db, issue, (pr, PR_SOURCE), now).await?,
            None => apply::apply(db, issue, &area, now).await?,
        }
    }
    close_ended(db, all, &open).await
}

/// Asks GitHub about each watched issue's item the search didn't return, and closes the closed
/// ones. One GitHub can't answer for stays as it is until the next sync.
async fn close_ended(db: &Db, all: &Settings, open: &HashSet<String>) -> AppResult<()> {
    let unseen = items::from_source(db, SOURCE)
        .await?
        .into_iter()
        .filter(|i| i.stage() != Stage::Done && all.node_for(&i.area) == all.node)
        .filter_map(|i| i.external_id)
        .filter(|ext| !open.contains(ext));
    for ext in unseen {
        if matches!(fate::closed(&ext).await, Ok(true)) {
            apply::close(db, &ext, Utc::now()).await?;
        }
    }
    Ok(())
}
