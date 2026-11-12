//! One-time import of the database from before Quarterdeck moved to Based: `quarterdeck.sqlite`
//! (plain `SQLite`) beside `quarterdeck.db`. The first start of a Based build copies its rows across
//! through the normal store functions, then renames it `quarterdeck.sqlite.imported`, so nothing is
//! deleted. Delete this module once every machine has run a Based build.

mod read;

use std::path::Path;

use chrono::Utc;

use super::{
    Db, activity, asks, commitments, events, items, meta, runs, sync_state, todos, urgent, watch,
};
use crate::error::AppResult;
use read::Old;

pub(in crate::store) const OLD_FILE: &str = "quarterdeck.sqlite";
const DONE: &str = "legacy.imported";

/// Imports the old database if there is one and it hasn't been imported. True if it ran.
pub async fn import(db: &Db, dir: &Path) -> AppResult<bool> {
    let path = dir.join(OLD_FILE);
    if !path.exists() || meta::text(db, DONE).await?.is_some() {
        return Ok(false);
    }
    let old = read::read(&path)?;
    write_items(db, &old).await?;
    write_records(db, &old).await?;
    write_state(db, &old).await?;
    meta::set_text(db, DONE, &Utc::now().to_rfc3339()).await?;
    set_aside(dir)?;
    Ok(true)
}

/// Items first: history, asks and watches belong to them.
async fn write_items(db: &Db, old: &Old) -> AppResult<()> {
    for (item, hidden) in &old.items {
        items::save(db, item).await?;
        if *hidden {
            items::hide(db, &item.id).await?;
        }
    }
    for (item_id, at, change) in &old.events {
        events::restore(db, item_id, *at, change).await?;
    }
    for (ask, answer) in &old.asks {
        let id = asks::insert(db, ask).await?;
        if let Some(answer) = answer {
            asks::answer(db, id, answer).await?;
        }
    }
    for (item_id, session) in &old.watches {
        watch::claim(db, item_id, session).await?;
    }
    Ok(())
}

async fn write_records(db: &Db, old: &Old) -> AppResult<()> {
    for c in &old.commitments {
        commitments::insert(db, c).await?;
    }
    for a in &old.activity {
        activity::insert(db, a).await?;
    }
    for r in &old.runs {
        runs::insert(db, r).await?;
    }
    for u in &old.urgent {
        urgent::raise(db, u).await?;
    }
    for t in &old.todos {
        todos::insert(db, t).await?;
    }
    Ok(())
}

/// Connector snapshots (so edits made since a sync stick) and app facts like the last tick.
async fn write_state(db: &Db, old: &Old) -> AppResult<()> {
    for (source, external_id, snapshot) in &old.sync_state {
        sync_state::set(db, source, external_id, snapshot).await?;
    }
    for (key, value) in &old.meta {
        match read::time(value) {
            Some(at) => meta::set_time(db, key, at).await?,
            None => meta::set_text(db, key, value).await?,
        }
    }
    Ok(())
}

/// Keeps the old file, and its write-ahead log, out of the way rather than deleting them.
fn set_aside(dir: &Path) -> AppResult<()> {
    for suffix in ["", "-wal", "-shm"] {
        let from = dir.join(format!("{OLD_FILE}{suffix}"));
        if from.exists() {
            std::fs::rename(&from, dir.join(format!("{OLD_FILE}.imported{suffix}")))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use based_runtime::rusqlite::Connection;

    use super::*;

    const ROWS: &str = "
        INSERT INTO items VALUES ('JD-1', 'Fix login', 'in review', 'in_review', 'review', 'Sam',
          'work', NULL, 'manual', NULL, 0, '2026-10-20 09:00:00.5+00:00', '2026-10-21 10:00:00+00:00', 0);
        INSERT INTO items VALUES ('app#7', 'Old PR', 'gone', 'up_next', NULL, 'you', 'work',
          'https://x', 'github', 'me/app#7', 1, '2026-10-20 09:00:00+00:00', '2026-10-20 09:00:00+00:00', 1);
        INSERT INTO item_events (item_id, at, change) VALUES ('JD-1', '2026-10-20 09:00:00+00:00', '{\"kind\":\"created\"}');
        INSERT INTO asks (item_id, question, context, options, session, cwd, answer)
          VALUES ('JD-1', 'Merge?', '', '[\"Yes\",\"No\"]', 's1', '/repo', NULL);
        INSERT INTO asks (item_id, question, context, options, answer) VALUES ('JD-1', '', '', '[]', 'go');
        INSERT INTO commitments (title, area, starts_at, ends_at, source, hidden)
          VALUES ('Standup', 'work', '2026-10-23 00:00:00+00:00', '2026-10-23 00:15:00+00:00', 'manual', 0);
        INSERT INTO activity (text, area, at, unread) VALUES ('Did a thing', 'work', '2026-10-21 08:00:00+00:00', 1);
        INSERT INTO runs (schedule_id, what, target, node, started_at, finished_at, outcome, summary)
          VALUES ('daily', 'Daily', '', 'home', '2026-10-21 08:00:00+00:00', '2026-10-21 08:01:00+00:00', 'ok', 'fine');
        INSERT INTO urgent (title, detail, area, raised_at) VALUES ('Help', 'now', 'work', '2026-10-21 08:00:00+00:00');
        INSERT INTO todos (title, area, due, done_at) VALUES ('Dentist', 'life', '2026-10-25', NULL);
        INSERT INTO watch (item_id, session, cwd, nudges, last_nudge) VALUES ('JD-1', 's1', '/repo', 1, NULL);
        INSERT INTO sync_state VALUES ('github', 'me/app#7', '{}');
        INSERT INTO meta VALUES ('scheduler.last_tick', '2026-10-21 08:00:00.25+00:00');
        INSERT INTO meta VALUES ('scheduler.last_error', '');
    ";

    fn old_database(dir: &Path) {
        let conn = Connection::open(dir.join(OLD_FILE)).unwrap();
        conn.execute_batch(include_str!("old_schema.sql")).unwrap();
        conn.execute_batch(ROWS).unwrap();
    }

    /// Every kind of record from `ROWS` is in the new database.
    async fn everything_arrived(db: &Db) {
        let item = items::get(db, "JD-1").await.unwrap();
        let waiting = crate::domain::life::Whose::Others {
            waiting_on: "Sam".to_owned(),
            then: None,
        };
        assert_eq!((item.life.whose(), item.rank), (Some(&waiting), 0));
        assert!(
            items::list(db)
                .await
                .unwrap()
                .iter()
                .all(|i| i.id != "app#7"),
            "hidden stays hidden"
        );
        assert_eq!(events::for_item(db, "JD-1").await.unwrap().len(), 1);
        assert_eq!(asks::list(db).await.unwrap().len(), 1);
        assert_eq!(asks::answered(db).await.unwrap().len(), 1);
        let watched = watch::all(db).await.unwrap();
        assert_eq!(watched.get("JD-1").map(|s| s.id.as_str()), Some("s1"));
        assert_eq!(todos::list(db).await.unwrap().len(), 1);
        assert_eq!(commitments::list(db).await.unwrap().len(), 1);
        assert_eq!(urgent::list(db).await.unwrap().len(), 1);
        assert_eq!(
            sync_state::get(db, "github", "me/app#7")
                .await
                .unwrap()
                .as_deref(),
            Some("{}")
        );
        assert!(
            meta::time(db, "scheduler.last_tick")
                .await
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn the_old_database_comes_across_once_and_is_kept_aside() {
        let dir = std::env::temp_dir().join(format!("qd-legacy-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        old_database(&dir);
        let db = Db::in_memory().await;
        assert!(import(&db, &dir).await.unwrap());

        everything_arrived(&db).await;
        assert!(!dir.join(OLD_FILE).exists());
        assert!(dir.join(format!("{OLD_FILE}.imported")).exists());
        old_database(&dir);
        assert!(!import(&db, &dir).await.unwrap(), "imports only once");
        drop(std::fs::remove_dir_all(&dir));
    }
}
