use chrono::{DateTime, Utc};

use super::client::{self, ItemRow};
use super::convert::{same, same_opt, text_key};
use super::db::{Db, gone_ok};
use super::time;
use crate::domain::item::WorkItem;
use crate::domain::life::{Life, LifeFields};
use crate::error::{AppError, AppResult};

/// Items made by hand or by an agent, as opposed to a connector.
pub const MANUAL: &str = "manual";

fn life(row: &ItemRow) -> AppResult<Life> {
    Ok(Life::from(LifeFields {
        stage: same(&row.stage)?,
        turn: same_opt(row.turn.as_ref())?,
        turn_until: time::get_opt(row.turn_until.as_deref())?,
        need: row.need.clone(),
        waiting_on: row.waiting_on.clone(),
        then_turn: same_opt(row.then_turn.as_ref())?,
        // A wake that no longer reads is dropped: the turn's deadline still brings it back.
        wake: row
            .wake
            .clone()
            .and_then(|w| serde_json::from_value(w).ok())
            .unwrap_or_default(),
    }))
}

fn from_row(row: ItemRow) -> AppResult<WorkItem> {
    Ok(WorkItem {
        life: life(&row)?,
        id: row.id.into_raw(),
        title: row.title,
        state_label: row.state_label,
        status: same_opt(row.status.as_ref())?,
        area: row.area,
        link: row.link,
        source: row.source,
        external_id: row.external_id,
        opened_at: time::get(&row.opened_at)?,
        state_since: time::get(&row.state_since)?,
        rank: row.rank,
    })
}

fn from_rows(rows: Vec<ItemRow>) -> AppResult<Vec<WorkItem>> {
    rows.into_iter().map(from_row).collect()
}

/// Visible items, highest priority first.
pub async fn list(db: &Db) -> AppResult<Vec<WorkItem>> {
    from_rows(db.api().items(client::ItemsInput {}, ()).await?)
}

pub async fn find(db: &Db, id: &str) -> AppResult<Option<WorkItem>> {
    let input = client::ItemInput { id: text_key(id) };
    let row = db.api().item(input, ()).await?;
    row.map(from_row).transpose()
}

pub async fn get(db: &Db, id: &str) -> AppResult<WorkItem> {
    find(db, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("item {id}")))
}

/// How many items there are, hidden ones included.
pub async fn count(db: &Db) -> AppResult<i64> {
    let row = db.api().item_count(client::ItemCountInput {}, ()).await?;
    Ok(row.map_or(0, |r| r.n))
}

/// The rank that puts a new item at the bottom of the priorities.
pub async fn next_rank(db: &Db) -> AppResult<i64> {
    let row = db.api().top_rank(client::TopRankInput {}, ()).await?;
    Ok(row.and_then(|r| r.top).map_or(0, |top| top + 1))
}

/// Inserts or replaces an item by id.
pub async fn save(db: &Db, item: &WorkItem) -> AppResult<()> {
    let life = LifeFields::from(item.life.clone());
    let input = client::SaveItemInput {
        id: text_key(&item.id),
        title: item.title.clone(),
        state_label: item.state_label.clone(),
        stage: same(&life.stage)?,
        status: same_opt(item.status.as_ref())?,
        turn: same_opt(life.turn.as_ref())?,
        turn_until: time::put_opt(life.turn_until),
        need: life.need,
        wake: (!life.wake.is_empty())
            .then(|| serde_json::to_value(&life.wake))
            .transpose()?,
        waiting_on: life.waiting_on,
        then_turn: same_opt(life.then_turn.as_ref())?,
        area: item.area.clone(),
        link: item.link.clone(),
        source: item.source.clone(),
        external_id: item.external_id.clone(),
        opened_at: time::put(item.opened_at),
        state_since: time::put(item.state_since),
        rank: item.rank,
    };
    Ok(db.api().save_item(input, ()).await?)
}

/// Deletes an item along with its asks, history and watch. Synced items are hidden instead, so
/// the next sync doesn't bring them back.
pub async fn delete(db: &Db, id: &str) -> AppResult<()> {
    if find(db, id).await?.is_some_and(|i| i.source != MANUAL) {
        return hide(db, id).await;
    }
    purge(db, id).await
}

/// Deletes an item along with its asks, history and watch, whatever made it.
pub async fn purge(db: &Db, id: &str) -> AppResult<()> {
    let input = client::DeleteItemInput { id: text_key(id) };
    gone_ok(
        db.api()
            .delete_item(input, ())
            .await
            .map_err(AppError::from),
    )
}

/// Ids of done items whose state has not changed since `before`.
pub async fn done_before(db: &Db, before: DateTime<Utc>) -> AppResult<Vec<String>> {
    let input = client::DoneItemsInput {
        before: time::put(before),
    };
    let rows = db.api().done_items(input, ()).await?;
    Ok(rows.into_iter().map(|r| r.id.into_raw()).collect())
}

/// Sets each item's rank to its position in `ids`, all at once. Ids that are gone are skipped.
pub async fn reorder(db: &Db, ids: &[String]) -> AppResult<()> {
    db.tx(|api| async move {
        for (rank, id) in (0_i64..).zip(ids) {
            let input = client::SetRankInput {
                id: text_key(id),
                rank,
            };
            match api.set_rank(input, ()).await {
                Err(e) if e.code() == "not_found" => {}
                other => other?,
            }
        }
        Ok(())
    })
    .await
}

/// The item a connector made for `external_id`, hidden or not.
pub async fn by_external(db: &Db, source: &str, external_id: &str) -> AppResult<Option<WorkItem>> {
    let input = client::ItemByExternalInput {
        source: source.to_owned(),
        external_id: external_id.to_owned(),
    };
    let row = db.api().item_by_external(input, ()).await?;
    row.map(from_row).transpose()
}

/// Every item a connector made, hidden or not.
pub async fn from_source(db: &Db, source: &str) -> AppResult<Vec<WorkItem>> {
    let input = client::ItemsFromSourceInput {
        source: source.to_owned(),
    };
    from_rows(db.api().items_from_source(input, ()).await?)
}

/// Hides a synced item so the connector doesn't bring it back.
pub async fn hide(db: &Db, id: &str) -> AppResult<()> {
    let input = client::HideItemInput { id: text_key(id) };
    Ok(db.api().hide_item(input, ()).await?)
}
