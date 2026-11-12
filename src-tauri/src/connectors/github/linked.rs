//! A hand-made item that links to a PR becomes that PR's item, so syncs keep it current and close
//! it when the PR ends, even one that merged before any sync saw it open.

use super::apply::SOURCE;
use super::link;
use crate::config::settings::Settings;
use crate::domain::item::{Stage, WorkItem};
use crate::error::AppResult;
use crate::store::{Db, items};

/// Binds each open hand-made item in an area this machine syncs to the PR its link names.
pub async fn bind(db: &Db, all: &Settings) -> AppResult<()> {
    for item in items::from_source(db, items::MANUAL).await? {
        if item.stage() == Stage::Done || all.node_for(&item.area) != all.node {
            continue;
        }
        if let Some(ext) = item.link.as_deref().and_then(pr_id) {
            take_over(db, item, ext).await?;
        }
    }
    Ok(())
}

/// `owner/repo#12` for a PR link.
fn pr_id(url: &str) -> Option<String> {
    let (repo, number) = link::parse(url, "pull")?;
    Some(format!("{repo}#{number}"))
}

async fn take_over(db: &Db, item: WorkItem, ext: String) -> AppResult<()> {
    // A sync made an item of its own for the PR before this one linked to it. Its snapshot is
    // the PR's, so it carries over and no event fires twice.
    if let Some(twin) = items::by_external(db, SOURCE, &ext).await?
        && twin.id != item.id
    {
        items::purge(db, &twin.id).await?;
    }
    let bound = WorkItem {
        source: SOURCE.to_owned(),
        external_id: Some(ext),
        ..item
    };
    items::save(db, &bound).await
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::domain::item::ItemInput;
    use crate::work;

    async fn save(db: &Db, id: &str, stage: Stage, link: &str) {
        let input = ItemInput {
            id: id.to_owned(),
            title: "Outages".to_owned(),
            state_label: "ready to merge".to_owned(),
            stage,
            status: None,
            area: "projects".to_owned(),
            link: Some(link.to_owned()),
        };
        work::save(db, input, Utc::now()).await.unwrap();
    }

    #[tokio::test]
    async fn open_items_linking_a_pr_become_its_item() {
        let db = Db::in_memory().await;
        save(
            &db,
            "B-1",
            Stage::InReview,
            "https://github.com/me/app/pull/24",
        )
        .await;
        save(&db, "B-2", Stage::Done, "https://github.com/me/app/pull/25").await;
        save(
            &db,
            "B-3",
            Stage::UpNext,
            "https://github.com/me/app/issues/9",
        )
        .await;
        // The item a sync made for the PR before B-1 linked to it.
        save(
            &db,
            "app#24",
            Stage::InReview,
            "https://github.com/me/app/pull/24",
        )
        .await;
        let twin = WorkItem {
            source: SOURCE.to_owned(),
            external_id: Some("me/app#24".to_owned()),
            ..items::get(&db, "app#24").await.unwrap()
        };
        items::save(&db, &twin).await.unwrap();

        bind(&db, &Settings::default()).await.unwrap();

        let bound = items::by_external(&db, SOURCE, "me/app#24")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(bound.id, "B-1");
        assert!(items::find(&db, "app#24").await.unwrap().is_none());
        assert_eq!(items::get(&db, "B-2").await.unwrap().source, items::MANUAL);
        assert_eq!(items::get(&db, "B-3").await.unwrap().source, items::MANUAL);
    }
}
