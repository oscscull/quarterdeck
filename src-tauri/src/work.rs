//! Creating and editing work items, recording each change in the item's history.

use chrono::{DateTime, Utc};

use crate::domain::event::Change;
use crate::domain::item::{ItemInput, WorkItem};
use crate::domain::life::{Life, Whose};
use crate::domain::turn::Turn;
use crate::error::AppResult;
use crate::lifecycle;
use crate::store::{Db, events, items};

/// Saves an item from the UI or an agent and returns its id. Unknown or empty ids create one.
pub async fn save(db: &Db, input: ItemInput, now: DateTime<Utc>) -> AppResult<String> {
    let existing = if input.id.is_empty() {
        None
    } else {
        items::find(db, &input.id).await?
    };
    match existing {
        Some(old) => update(db, &old, input, now).await,
        None => create(db, input, now).await,
    }
}

async fn create(db: &Db, input: ItemInput, now: DateTime<Utc>) -> AppResult<String> {
    let id = if input.id.trim().is_empty() {
        fresh_id(db).await?
    } else {
        input.id.trim().to_owned()
    };
    let item = WorkItem {
        id: id.clone(),
        opened_at: now,
        state_since: now,
        rank: items::next_rank(db).await?,
        life: lifecycle::new(input.stage, &input.state_label),
        ..from_input(input)
    };
    items::save(db, &item).await?;
    events::record(db, &id, now, &Change::Created).await?;
    Ok(id)
}

async fn update(
    db: &Db,
    old: &WorkItem,
    input: ItemInput,
    now: DateTime<Utc>,
) -> AppResult<String> {
    let life = if input.stage == old.stage() {
        old.life.clone()
    } else {
        lifecycle::moved(&old.life, input.stage, &input.state_label)
    };
    let mut item = WorkItem {
        id: old.id.clone(),
        opened_at: old.opened_at,
        state_since: old.state_since,
        rank: old.rank,
        source: old.source.clone(),
        external_id: old.external_id.clone(),
        life,
        ..from_input(input)
    };
    let changes = diff(old, &item);
    if !changes.is_empty() {
        item.state_since = now;
    }
    items::save(db, &item).await?;
    for change in &changes {
        events::record(db, &item.id, now, change).await?;
    }
    Ok(item.id)
}

/// The fields filled from input; ids, times, rank and life are set by the caller.
fn from_input(input: ItemInput) -> WorkItem {
    WorkItem {
        id: String::new(),
        title: input.title,
        state_label: input.state_label,
        life: Life::not_started(),
        status: input.status,
        area: input.area,
        link: input.link.filter(|l| !l.trim().is_empty()),
        source: items::MANUAL.to_owned(),
        external_id: None,
        opened_at: DateTime::<Utc>::MIN_UTC,
        state_since: DateTime::<Utc>::MIN_UTC,
        rank: 0,
    }
}

/// Changes worth a line in the history. Title and link edits are housekeeping.
fn diff(old: &WorkItem, new: &WorkItem) -> Vec<Change> {
    let mut changes = Vec::new();
    if old.stage() != new.stage() {
        changes.push(Change::Stage { stage: new.stage() });
    }
    if old.life.whose() != new.life.whose()
        && let Some(Whose::You { need }) = new.life.whose()
    {
        changes.push(Change::Turn {
            turn: Turn::You,
            why: need.clone(),
        });
    }
    if old.status != new.status {
        changes.push(Change::Status { status: new.status });
    }
    if old.state_label != new.state_label {
        changes.push(Change::Label {
            label: new.state_label.clone(),
        });
    }
    changes
}

/// "B-1", "B-2"… for items made without a ticket key.
async fn fresh_id(db: &Db) -> AppResult<String> {
    let mut n = items::count(db).await? + 1;
    while items::find(db, &format!("B-{n}")).await?.is_some() {
        n += 1;
    }
    Ok(format!("B-{n}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::item::{Stage, Status};

    fn input(id: &str, stage: Stage) -> ItemInput {
        ItemInput {
            id: id.to_owned(),
            title: "Thing".to_owned(),
            state_label: "starting".to_owned(),
            stage,
            status: Some(Status::Review),
            area: "work".to_owned(),
            link: Some(String::new()),
        }
    }

    #[tokio::test]
    async fn creates_with_generated_ids_at_the_bottom() {
        let db = Db::in_memory().await;
        let now = Utc::now();
        let first = save(&db, input("", Stage::UpNext), now).await.unwrap();
        assert_eq!(first, "B-1");
        let keyed = save(&db, input("JD-1", Stage::UpNext), now).await.unwrap();
        assert_eq!(keyed, "JD-1");
        let listed = items::list(&db).await.unwrap();
        let ranks: Vec<i64> = listed.iter().map(|i| i.rank).collect();
        assert_eq!(ranks, [0, 1]);
        assert_eq!(items::get(&db, "B-1").await.unwrap().link, None);
    }

    #[tokio::test]
    async fn records_stage_changes_only_when_they_happen_and_starting_hands_over() {
        let db = Db::in_memory().await;
        let id = save(&db, input("", Stage::UpNext), Utc::now())
            .await
            .unwrap();
        save(&db, input(&id, Stage::UpNext), Utc::now())
            .await
            .unwrap();
        save(&db, input(&id, Stage::InReview), Utc::now())
            .await
            .unwrap();
        let history: Vec<Change> = events::for_item(&db, &id)
            .await
            .unwrap()
            .into_iter()
            .map(|e| e.change)
            .collect();
        assert_eq!(
            history,
            [
                Change::Turn {
                    turn: Turn::You,
                    why: "starting".to_owned()
                },
                Change::Stage {
                    stage: Stage::InReview
                },
                Change::Created
            ]
        );
    }
}
