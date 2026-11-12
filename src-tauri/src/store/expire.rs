//! Deletes stored agent output once it has outlived its TTL.

use chrono::{DateTime, Duration, Utc};

use super::client;
use super::db::Db;
use super::{events, fired, items, time};
use crate::error::AppResult;

/// Meetings are only useful until shortly after they end.
const PAST_MEETINGS: Duration = Duration::days(1);

/// How long each kind of stored output lives.
#[derive(Debug, Clone, Copy)]
pub struct Ttl {
    pub activity: Duration,
    pub runs: Duration,
    pub events: Duration,
    pub done_todos: Duration,
    pub done_items: Duration,
    /// Bookkeeping in item history.
    pub trace: Duration,
}

/// Removes stored output past its TTL: activity, finished runs, events, done to-dos, past meetings and
/// their alerts and calendar snapshots, history bookkeeping, and done items with their history. Live
/// runs are kept.
pub async fn expire(db: &Db, now: DateTime<Utc>, ttl: Ttl) -> AppResult<()> {
    let api = db.api();
    let before = |age: Duration| time::put(now - age);
    api.expire_activity(
        client::ExpireActivityInput {
            before: before(ttl.activity),
        },
        (),
    )
    .await?;
    api.expire_runs(
        client::ExpireRunsInput {
            before: before(ttl.runs),
        },
        (),
    )
    .await?;
    api.expire_todos(
        client::ExpireTodosInput {
            before: before(ttl.done_todos),
        },
        (),
    )
    .await?;
    api.expire_commitments(
        client::ExpireCommitmentsInput {
            before: before(PAST_MEETINGS),
        },
        (),
    )
    .await?;
    api.expire_sync_snapshots(client::ExpireSyncSnapshotsInput {}, ())
        .await?;
    fired::expire(db, now - ttl.events).await?;
    events::expire_trace(db, now - ttl.trace).await?;
    for id in items::done_before(db, now - ttl.done_items).await? {
        items::delete(db, &id).await?;
    }
    Ok(())
}
