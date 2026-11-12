use super::client::{self, UrgentRow};
use super::convert::{int, key};
use super::db::{Db, gone_ok};
use super::time;
use crate::domain::urgent::Urgent;
use crate::error::{AppError, AppResult};

fn from_row(row: UrgentRow) -> AppResult<Urgent> {
    Ok(Urgent {
        id: int(&row.id)?,
        title: row.title,
        detail: row.detail,
        area: row.area,
        raised_at: time::get(&row.raised_at)?,
        schedule_id: row.schedule_id,
        run_id: row.run_id,
    })
}

/// Open urgent items, newest first.
pub async fn list(db: &Db) -> AppResult<Vec<Urgent>> {
    let rows = db
        .api()
        .urgent_items(client::UrgentItemsInput {}, ())
        .await?;
    rows.into_iter().map(from_row).collect()
}

pub async fn find(db: &Db, id: i64) -> AppResult<Urgent> {
    let input = client::UrgentInput { id: key(id) };
    let row = db.api().urgent(input, ()).await?;
    from_row(row.ok_or_else(|| AppError::NotFound(format!("urgent item {id}")))?)
}

/// Stores a new urgent item and returns its id; the `id` given is ignored.
pub async fn raise(db: &Db, item: &Urgent) -> AppResult<i64> {
    let input = client::RaiseUrgentInput {
        title: item.title.clone(),
        detail: item.detail.clone(),
        area: item.area.clone(),
        raised_at: time::put(item.raised_at),
        schedule_id: item.schedule_id.clone(),
        run_id: item.run_id,
    };
    int(&db.api().raise_urgent(input, ()).await?.id)
}

/// Dismissing deletes: an urgent item has no life after it is dealt with.
pub async fn dismiss(db: &Db, id: i64) -> AppResult<()> {
    let input = client::DismissUrgentInput { id: key(id) };
    gone_ok(
        db.api()
            .dismiss_urgent(input, ())
            .await
            .map_err(AppError::from),
    )
}

pub async fn dismiss_all(db: &Db) -> AppResult<()> {
    let input = client::DismissAllUrgentInput {};
    Ok(db.api().dismiss_all_urgent(input, ()).await?)
}
