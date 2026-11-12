//! An item whose PR Quarterdeck opened (an issue it fixed) becomes that PR's item, so syncs update
//! it rather than adding a second one.

use super::apply::SOURCE;
use crate::domain::activity::Activity;
use crate::domain::event::Change;
use crate::domain::item::{ItemInput, Stage, Status, WorkItem};
use crate::domain::turn::Turn;
use crate::error::AppResult;
use crate::store::{Db, activity, events, items};
use crate::turns::{self, Clock, Pass};
use crate::work;

/// The PR that the item's work became.
#[derive(Debug, Clone, Copy)]
pub struct OpenedPr<'a> {
    /// `owner/repo`.
    pub repo: &'a str,
    pub number: u64,
    pub url: &'a str,
}

pub async fn adopt(db: &Db, item_id: &str, pr: OpenedPr<'_>, clock: Clock<'_>) -> AppResult<()> {
    let now = clock.now;
    let ext = format!("{}#{}", pr.repo, pr.number);
    // A sync between opening the PR and now made an item of its own for it.
    if let Some(twin) = items::by_external(db, SOURCE, &ext).await?
        && twin.id != item_id
    {
        items::purge(db, &twin.id).await?;
    }
    let item = items::get(db, item_id).await?;
    let input = ItemInput {
        id: item.id.clone(),
        title: item.title.clone(),
        state_label: format!("PR #{} open", pr.number),
        stage: Stage::InReview,
        status: Some(Status::Ci),
        area: item.area.clone(),
        link: Some(pr.url.to_owned()),
    };
    work::save(db, input, now).await?;
    let synced = WorkItem {
        source: SOURCE.to_owned(),
        external_id: Some(ext),
        ..items::get(db, item_id).await?
    };
    items::save(db, &synced).await?;
    let text = format!("Quarterdeck opened {}", pr.url);
    events::record(db, item_id, now, &Change::Note { text }).await?;
    let pass = Pass::to(Turn::Others, format!("opened PR #{}", pr.number)).waiting_on("CI");
    turns::pass(db, item_id, pass, clock).await?;
    let update = Activity {
        id: 0,
        text: format!("Opened PR #{} for {item_id}: {}", pr.number, item.title),
        area: item.area,
        at: now,
        unread: true,
    };
    activity::insert(db, &update).await
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::config::turns::TurnSettings;

    async fn save(db: &Db, id: &str, link: &str) {
        let input = ItemInput {
            id: id.to_owned(),
            title: "Upsert breaks".to_owned(),
            state_label: "up next".to_owned(),
            stage: Stage::UpNext,
            status: None,
            area: "projects".to_owned(),
            link: Some(link.to_owned()),
        };
        work::save(db, input, Utc::now()).await.unwrap();
    }

    #[tokio::test]
    async fn the_issue_item_becomes_the_prs_and_its_twin_goes() {
        let db = Db::in_memory().await;
        save(
            &db,
            "baseddsl#178",
            "https://github.com/o/baseddsl/issues/178",
        )
        .await;
        save(
            &db,
            "baseddsl#190",
            "https://github.com/o/baseddsl/pull/190",
        )
        .await;
        let twin = WorkItem {
            source: SOURCE.to_owned(),
            external_id: Some("o/baseddsl#190".to_owned()),
            ..items::get(&db, "baseddsl#190").await.unwrap()
        };
        items::save(&db, &twin).await.unwrap();

        let pr = OpenedPr {
            repo: "o/baseddsl",
            number: 190,
            url: "https://github.com/o/baseddsl/pull/190",
        };
        let limits = TurnSettings::default();
        adopt(&db, "baseddsl#178", pr, Clock::now(&limits))
            .await
            .unwrap();

        let item = items::by_external(&db, SOURCE, "o/baseddsl#190")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(item.id, "baseddsl#178");
        assert_eq!(
            (item.stage(), item.turn()),
            (Stage::InReview, Some(Turn::Others))
        );
        assert_eq!(item.link.as_deref(), Some(pr.url));
        assert!(items::find(&db, "baseddsl#190").await.unwrap().is_none());
    }
}
