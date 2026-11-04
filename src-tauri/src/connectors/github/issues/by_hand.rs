//! Items an agent made by hand for an issue, whatever their id, so the sync takes them over instead of
//! adding a twin.

use super::apply::{SOURCE, untriaged};
use super::fetch::Issue;
use crate::connectors::github::link;
use crate::domain::item::{Stage, WorkItem};
use crate::error::AppResult;
use crate::store::{Db, items};

/// An open item made by hand that links the issue.
async fn find(db: &Db, issue: &Issue) -> AppResult<Option<WorkItem>> {
    let wanted = Some((issue.repo.clone(), issue.number));
    Ok(items::list(db).await?.into_iter().find(|i| {
        i.source == items::MANUAL
            && i.stage() != Stage::Done
            && i.link.as_deref().and_then(|l| link::parse(l, "issues")) == wanted
    }))
}

/// Makes a hand-made item for the issue its synced item. A synced twin nobody picked up goes; one
/// someone did is kept, and the hand-made item stays as it is.
pub async fn take_over(db: &Db, issue: &Issue) -> AppResult<()> {
    let Some(by_hand) = find(db, issue).await? else {
        return Ok(());
    };
    let ext = issue.external_id();
    if let Some(twin) = items::by_external(db, SOURCE, &ext).await? {
        if !untriaged(&twin) {
            return Ok(());
        }
        items::purge(db, &twin.id).await?;
    }
    let synced = WorkItem {
        source: SOURCE.to_owned(),
        external_id: Some(ext),
        ..by_hand
    };
    items::save(db, &synced).await
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::connectors::github::issues::apply::apply;
    use crate::domain::item::ItemInput;
    use crate::work;

    fn issue() -> Issue {
        Issue {
            repo: "me/app".to_owned(),
            number: 7,
            title: "Breaks".to_owned(),
            url: "https://github.com/me/app/issues/7".to_owned(),
            labels: Vec::new(),
        }
    }

    async fn by_hand(db: &Db) {
        let input = ItemInput {
            id: "B-47".to_owned(),
            title: "App: fix the break".to_owned(),
            state_label: "Scoping".to_owned(),
            stage: Stage::UpNext,
            status: None,
            area: "projects".to_owned(),
            link: Some("https://github.com/me/app/issues/7#top".to_owned()),
        };
        work::save(db, input, Utc::now()).await.unwrap();
    }

    async fn ids(db: &Db) -> Vec<String> {
        items::list(db)
            .await
            .unwrap()
            .into_iter()
            .map(|i| i.id)
            .collect()
    }

    #[tokio::test]
    async fn an_item_made_by_hand_under_any_id_is_taken_over() {
        let db = Db::in_memory().await;
        by_hand(&db).await;
        apply(&db, &issue(), "projects", Utc::now()).await.unwrap();
        assert_eq!(ids(&db).await, ["B-47"]);
        let item = items::by_external(&db, SOURCE, "me/app#7").await.unwrap();
        assert_eq!(item.map(|i| i.id).as_deref(), Some("B-47"));
    }

    #[tokio::test]
    async fn a_twin_the_sync_already_made_goes() {
        let db = Db::in_memory().await;
        apply(&db, &issue(), "projects", Utc::now()).await.unwrap();
        by_hand(&db).await;
        apply(&db, &issue(), "projects", Utc::now()).await.unwrap();
        assert_eq!(ids(&db).await, ["B-47"]);
    }
}
