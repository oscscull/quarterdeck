//! Writes PRs into Quarterdeck as items, respecting edits made since the last sync.

use std::collections::HashSet;

use chrono::{DateTime, Utc};

use super::fate::Fate;
use super::fetch::Pr;
use super::snapshot::{self, Snapshot};
use super::turn;
use crate::config::turns::TurnSettings;
use crate::domain::event::Change;
use crate::domain::item::{ItemInput, Stage, WorkItem};
use crate::domain::schedule::EventKind;
use crate::error::AppResult;
use crate::store::{Db, events, items, sync_state};
use crate::turns::Clock;
use crate::work;

pub const SOURCE: &str = "github";

pub fn external_id(pr: &Pr) -> String {
    format!("{}#{}", pr.repo, pr.number)
}

/// A short display id, e.g. `app#12`.
fn display_id(pr: &Pr) -> String {
    let name = pr
        .repo
        .rsplit_once('/')
        .map_or(pr.repo.as_str(), |(_, n)| n);
    format!("{name}#{}", pr.number)
}

fn input(id: String, s: &Snapshot, area: String, link: Option<String>) -> ItemInput {
    ItemInput {
        id,
        title: s.owned.title.clone(),
        state_label: s.owned.label.clone(),
        stage: s.owned.stage,
        status: s.owned.status,
        area,
        link,
    }
}

/// Creates or updates the item for one PR and returns the events it fired.
pub async fn apply(
    db: &Db,
    pr: &Pr,
    (area, limits): (&str, &TurnSettings),
    now: DateTime<Utc>,
) -> AppResult<Vec<EventKind>> {
    let ext = external_id(pr);
    let new = snapshot::of(pr);
    let old: Option<Snapshot> = sync_state::get(db, SOURCE, &ext)
        .await?
        .and_then(|json| serde_json::from_str(&json).ok());
    let existing = items::by_external(db, SOURCE, &ext).await?;
    // An item that has the PR but no snapshot of it got the PR when Quarterdeck opened it.
    let adopted = existing.is_some() && old.is_none();
    match existing {
        None => {
            let id = create(db, pr, area, now).await?;
            turn::follow(db, &id, &new.owned, Clock { limits, now }).await?;
        }
        Some(item) if old.as_ref().map(|o| &o.owned) != Some(&new.owned) => {
            let id = work::save(db, input(item.id, &new, item.area, item.link), now).await?;
            turn::follow(db, &id, &new.owned, Clock { limits, now }).await?;
        }
        Some(_) => {}
    }
    note_changes(db, &ext, (old.as_ref(), &new), now).await?;
    sync_state::set(db, SOURCE, &ext, &serde_json::to_string(&new)?).await?;
    Ok(if adopted {
        snapshot::adopted(&new)
    } else {
        snapshot::events(old.as_ref(), &new)
    })
}

/// Adds informational check results to the item's history the first time each is seen.
async fn note_changes(
    db: &Db,
    ext: &str,
    (old, new): (Option<&Snapshot>, &Snapshot),
    now: DateTime<Utc>,
) -> AppResult<()> {
    let Some(item) = items::by_external(db, SOURCE, ext).await? else {
        return Ok(());
    };
    let seen = old.map(|o| o.notes.as_slice()).unwrap_or_default();
    for text in new.notes.iter().filter(|n| !seen.contains(n)) {
        events::record(db, &item.id, now, &Change::Note { text: text.clone() }).await?;
    }
    Ok(())
}

async fn create(db: &Db, pr: &Pr, area: &str, now: DateTime<Utc>) -> AppResult<String> {
    let s = &snapshot::of(pr);
    let fields = input(display_id(pr), s, area.to_owned(), Some(pr.url.clone()));
    let id = work::save(db, fields, now).await?;
    let item = WorkItem {
        source: SOURCE.to_owned(),
        external_id: Some(external_id(pr)),
        ..items::get(db, &id).await?
    };
    items::save(db, &item).await?;
    Ok(id)
}

/// External ids of open GitHub items whose PR wasn't in the open-PR search, to ask GitHub about.
pub async fn unseen(db: &Db, open: &HashSet<String>) -> AppResult<Vec<String>> {
    Ok(items::from_source(db, SOURCE)
        .await?
        .into_iter()
        .filter(|i| i.stage() != Stage::Done)
        .filter_map(|i| i.external_id)
        .filter(|ext| !open.contains(ext))
        .collect())
}

/// Marks the item for a PR done, once GitHub says it was merged or closed. Open PRs stay as they are.
pub async fn close(db: &Db, ext: &str, fate: Fate, now: DateTime<Utc>) -> AppResult<()> {
    let label = match fate {
        Fate::Open => return Ok(()),
        Fate::Merged => "merged",
        Fate::Closed => "closed without merging",
    };
    let Some(item) = items::by_external(db, SOURCE, ext).await? else {
        return Ok(());
    };
    let done = ItemInput {
        id: item.id,
        title: item.title,
        state_label: label.to_owned(),
        stage: Stage::Done,
        status: None,
        area: item.area,
        link: item.link,
    };
    work::save(db, done, now).await?;
    sync_state::forget(db, SOURCE, ext).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pr(comments: u64) -> Pr {
        Pr {
            repo: "me/app".to_owned(),
            number: 7,
            title: "Fix login".to_owned(),
            url: "https://github.com/me/app/pull/7".to_owned(),
            draft: false,
            review: None,
            ci: Some("SUCCESS".to_owned()),
            comments,
            reviewers: super::super::fetch::Reviewers::Requested,
            reviewer_names: Vec::new(),
            closes: Vec::new(),
            base: super::super::fetch::Base::Current,
            checks: Vec::new(),
            notes: Vec::new(),
        }
    }

    #[tokio::test]
    async fn syncs_keep_edits_until_github_changes() {
        let db = Db::in_memory().await;
        let limits = TurnSettings::default();
        apply(&db, &pr(0), ("work", &limits), Utc::now())
            .await
            .unwrap();
        let waiting = items::get(&db, "app#7").await.unwrap();
        let reviewers = crate::domain::life::Whose::Others {
            waiting_on: "reviewers".to_owned(),
            then: None,
        };
        assert_eq!(waiting.life.whose(), Some(&reviewers));
        let mut edited = items::get(&db, "app#7").await.unwrap();
        edited.state_label = "Sam is on holiday".to_owned();
        items::save(&db, &edited).await.unwrap();
        let events = apply(&db, &pr(1), ("work", &limits), Utc::now())
            .await
            .unwrap();
        assert_eq!(events, [EventKind::PrNewComments]);
        assert_eq!(
            items::get(&db, "app#7").await.unwrap().state_label,
            "Sam is on holiday"
        );
        // Missing from the search alone closes nothing; GitHub saying so does.
        assert_eq!(unseen(&db, &HashSet::new()).await.unwrap(), ["me/app#7"]);
        close(&db, "me/app#7", Fate::Open, Utc::now())
            .await
            .unwrap();
        assert_eq!(
            items::get(&db, "app#7").await.unwrap().stage(),
            Stage::InReview
        );
        close(&db, "me/app#7", Fate::Merged, Utc::now())
            .await
            .unwrap();
        let merged = items::get(&db, "app#7").await.unwrap();
        assert_eq!(
            (merged.stage(), merged.state_label.as_str()),
            (Stage::Done, "merged")
        );
    }

    #[tokio::test]
    async fn a_pr_quarterdeck_opened_reports_ci_that_failed_before_the_first_sync() {
        let db = Db::in_memory().await;
        let limits = TurnSettings::default();
        let failing = Pr {
            ci: Some("FAILURE".to_owned()),
            ..pr(0)
        };
        let fresh = Pr {
            number: 8,
            ..failing.clone()
        };
        let quiet = apply(&db, &fresh, ("work", &limits), Utc::now()).await;
        assert_eq!(quiet.unwrap(), []);
        let input = ItemInput {
            id: "app#3".to_owned(),
            title: "Login breaks".to_owned(),
            state_label: "PR #7 open".to_owned(),
            stage: Stage::InReview,
            status: None,
            area: "work".to_owned(),
            link: None,
        };
        work::save(&db, input, Utc::now()).await.unwrap();
        let adopted = WorkItem {
            source: SOURCE.to_owned(),
            external_id: Some("me/app#7".to_owned()),
            ..items::get(&db, "app#3").await.unwrap()
        };
        items::save(&db, &adopted).await.unwrap();
        let events = apply(&db, &failing, ("work", &limits), Utc::now()).await;
        assert_eq!(events.unwrap(), [EventKind::CiFailed]);
    }
}
