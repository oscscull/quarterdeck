//! Writes issues into Quarterdeck as items to triage, respecting edits made since the last sync.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::by_hand;
use super::fetch::Issue;
use crate::domain::event::Change;
use crate::domain::item::{ItemInput, Stage, WorkItem};
use crate::error::AppResult;
use crate::store::{Db, events, items, sync_state};
use crate::work;

pub const SOURCE: &str = "github-issue";

/// The label an issue's item gets until someone decides what to do with it.
const TRIAGE: &str = "needs triage";

/// What the last sync saw. Only a title change upstream is re-applied, so edits stick.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Snapshot {
    title: String,
    /// The PR (`owner/repo#12`) that closes it, while it has one open.
    #[serde(default)]
    covered_by: Option<String>,
}

/// An issue's item that nobody has picked up: as synced, its turn never passed.
pub fn untriaged(item: &WorkItem) -> bool {
    item.source == SOURCE && item.life.untouched() && item.status.is_none()
}

async fn snapshot(db: &Db, ext: &str) -> AppResult<Option<Snapshot>> {
    Ok(sync_state::get(db, SOURCE, ext)
        .await?
        .and_then(|json| serde_json::from_str(&json).ok()))
}

async fn remember(db: &Db, ext: &str, s: &Snapshot) -> AppResult<()> {
    sync_state::set(db, SOURCE, ext, &serde_json::to_string(s)?).await
}

fn input(item: &WorkItem, title: String, stage: Stage, label: String) -> ItemInput {
    ItemInput {
        id: item.id.clone(),
        title,
        state_label: label,
        stage,
        status: item.status,
        area: item.area.clone(),
        link: item.link.clone(),
    }
}

/// Creates or updates the item for one open issue.
pub async fn apply(db: &Db, issue: &Issue, area: &str, now: DateTime<Utc>) -> AppResult<()> {
    let ext = issue.external_id();
    by_hand::take_over(db, issue).await?;
    let old = snapshot(db, &ext).await?;
    match items::by_external(db, SOURCE, &ext).await? {
        None => create(db, issue, area, now).await?,
        // GitHub closed it (which forgets the snapshot) and it is open again. One the user marked
        // done here stays done.
        Some(item) if item.stage() == Stage::Done && old.is_none() => {
            let reopened = input(
                &item,
                issue.title.clone(),
                Stage::UpNext,
                "reopened".to_owned(),
            );
            work::save(db, reopened, now).await?;
        }
        Some(item)
            if item.stage() != Stage::Done
                && old.as_ref().is_none_or(|o| o.title != issue.title) =>
        {
            let retitled = input(
                &item,
                issue.title.clone(),
                item.stage(),
                item.state_label.clone(),
            );
            work::save(db, retitled, now).await?;
        }
        Some(_) => {}
    }
    let new = Snapshot {
        title: issue.title.clone(),
        covered_by: None,
    };
    remember(db, &ext, &new).await
}

/// `baseddsl#12`, or the full `owner/repo#12` if another item already has the short id.
async fn free_id(db: &Db, issue: &Issue) -> AppResult<String> {
    let name = issue
        .repo
        .rsplit_once('/')
        .map_or(issue.repo.as_str(), |(_, n)| n);
    let short = format!("{name}#{}", issue.number);
    match items::find(db, &short).await? {
        Some(other)
            if other.source != items::MANUAL
                || other.link.as_deref() != Some(issue.url.as_str()) =>
        {
            Ok(issue.external_id())
        }
        Some(_) | None => Ok(short),
    }
}

/// Makes the issue's item, or takes over one an agent already made for it by hand.
async fn create(db: &Db, issue: &Issue, area: &str, now: DateTime<Utc>) -> AppResult<()> {
    let id = free_id(db, issue).await?;
    if items::find(db, &id).await?.is_none() {
        let fields = ItemInput {
            id: id.clone(),
            title: issue.title.clone(),
            state_label: TRIAGE.to_owned(),
            stage: Stage::UpNext,
            status: None,
            area: area.to_owned(),
            link: Some(issue.url.clone()),
        };
        work::save(db, fields, now).await?;
    }
    let item = WorkItem {
        source: SOURCE.to_owned(),
        external_id: Some(issue.external_id()),
        ..items::get(db, &id).await?
    };
    items::save(db, &item).await
}

/// One of the user's open PRs closes this issue, so the PR's item stands for it. An untouched
/// item for the issue goes; one someone picked up stays. The PR's item notes the issue once.
pub async fn cover(db: &Db, issue: &Issue, pr: (&str, &str), now: DateTime<Utc>) -> AppResult<()> {
    let ext = issue.external_id();
    let (pr_ext, pr_source) = pr;
    let old = snapshot(db, &ext).await?.unwrap_or_default();
    if old.covered_by.as_deref() == Some(pr_ext) {
        return Ok(());
    }
    if let Some(item) = items::by_external(db, SOURCE, &ext).await?
        && untriaged(&item)
    {
        items::purge(db, &item.id).await?;
    }
    if let Some(pr_item) = items::by_external(db, pr_source, pr_ext).await? {
        let text = format!("Closes issue {ext}: {}", issue.title);
        events::record(db, &pr_item.id, now, &Change::Note { text }).await?;
    }
    let new = Snapshot {
        title: issue.title.clone(),
        covered_by: Some(pr_ext.to_owned()),
    };
    remember(db, &ext, &new).await
}

/// Marks an issue's item done once GitHub says the issue closed.
pub async fn close(db: &Db, ext: &str, now: DateTime<Utc>) -> AppResult<()> {
    if let Some(item) = items::by_external(db, SOURCE, ext).await? {
        let done = ItemInput {
            status: None,
            ..input(&item, item.title.clone(), Stage::Done, "closed".to_owned())
        };
        work::save(db, done, now).await?;
    }
    sync_state::forget(db, SOURCE, ext).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::life::{Life, Phase, Whose};
    use crate::domain::turn::Turn;

    fn issue(title: &str) -> Issue {
        Issue {
            repo: "me/app".to_owned(),
            number: 7,
            title: title.to_owned(),
            url: "https://github.com/me/app/issues/7".to_owned(),
            labels: Vec::new(),
        }
    }

    async fn the_item(db: &Db) -> Option<WorkItem> {
        items::by_external(db, SOURCE, "me/app#7").await.unwrap()
    }

    #[tokio::test]
    async fn issues_arrive_to_triage_and_keep_edits() {
        let db = Db::in_memory().await;
        apply(&db, &issue("Breaks"), "projects", Utc::now())
            .await
            .unwrap();
        let mut item = the_item(&db).await.unwrap();
        assert_eq!(item.id, "app#7");
        assert!(untriaged(&item));
        item.life = Life::open_as(Phase::InProgress, Whose::Agent, Some(Utc::now()));
        items::save(&db, &item).await.unwrap();
        apply(&db, &issue("Breaks"), "projects", Utc::now())
            .await
            .unwrap();
        assert!(the_item(&db).await.unwrap().is_turn(Turn::Agent));
        apply(&db, &issue("Breaks badly"), "projects", Utc::now())
            .await
            .unwrap();
        let retitled = the_item(&db).await.unwrap();
        assert_eq!(
            (retitled.title.as_str(), retitled.turn()),
            ("Breaks badly", Some(Turn::Agent))
        );
        close(&db, "me/app#7", Utc::now()).await.unwrap();
        assert_eq!(the_item(&db).await.unwrap().stage(), Stage::Done);
        apply(&db, &issue("Breaks badly"), "projects", Utc::now())
            .await
            .unwrap();
        assert_eq!(the_item(&db).await.unwrap().stage(), Stage::UpNext);
    }

    #[tokio::test]
    async fn an_item_made_by_hand_for_the_issue_is_taken_over() {
        let db = Db::in_memory().await;
        let by_hand = ItemInput {
            id: "app#7".to_owned(),
            title: "Breaks".to_owned(),
            state_label: "Claude on it".to_owned(),
            stage: Stage::InProgress,
            status: None,
            area: "projects".to_owned(),
            link: Some("https://github.com/me/app/issues/7".to_owned()),
        };
        work::save(&db, by_hand, Utc::now()).await.unwrap();
        apply(&db, &issue("Breaks"), "work", Utc::now())
            .await
            .unwrap();
        let item = the_item(&db).await.unwrap();
        assert_eq!(
            (item.id.as_str(), item.stage()),
            ("app#7", Stage::InProgress)
        );
    }

    #[tokio::test]
    async fn a_pr_that_closes_the_issue_stands_for_it() {
        let db = Db::in_memory().await;
        apply(&db, &issue("Breaks"), "projects", Utc::now())
            .await
            .unwrap();
        cover(&db, &issue("Breaks"), ("me/app#9", "github"), Utc::now())
            .await
            .unwrap();
        assert!(the_item(&db).await.is_none());
        // The PR closed without merging: the issue comes back to triage.
        apply(&db, &issue("Breaks"), "projects", Utc::now())
            .await
            .unwrap();
        assert!(untriaged(&the_item(&db).await.unwrap()));
        // One someone started stays.
        let mut started = the_item(&db).await.unwrap();
        started.life = Life::open_as(Phase::InProgress, Whose::Agent, None);
        items::save(&db, &started).await.unwrap();
        cover(&db, &issue("Breaks"), ("me/app#10", "github"), Utc::now())
            .await
            .unwrap();
        assert!(the_item(&db).await.is_some());
    }
}
