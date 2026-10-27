//! Reads every table of the pre-Based database into the app's own types.

use std::path::Path;

use based_runtime::rusqlite::{Connection, OpenFlags, Row, types::Type};
use chrono::{DateTime, NaiveDate, Utc};
use serde::de::DeserializeOwned;

use crate::domain::activity::Activity;
use crate::domain::ask::Ask;
use crate::domain::commitment::Commitment;
use crate::domain::event::Change;
use crate::domain::item::{Status, WorkItem};
use crate::domain::life::{Life, LifeFields};
use crate::domain::run::Run;
use crate::domain::session::SessionRef;
use crate::domain::todo::Todo;
use crate::domain::turn::Turn;
use crate::domain::urgent::Urgent;
use crate::error::{AppError, AppResult};

type Sql<T> = based_runtime::rusqlite::Result<T>;

/// One line of an item's history: its item, when, and what changed.
pub type OldEvent = (String, DateTime<Utc>, Change);

/// Everything worth keeping. Run requests aren't: they were for the old app's next tick.
#[derive(Debug, Default)]
pub struct Old {
    pub items: Vec<(WorkItem, bool)>,
    pub events: Vec<OldEvent>,
    pub asks: Vec<(Ask, Option<String>)>,
    pub commitments: Vec<Commitment>,
    pub activity: Vec<Activity>,
    pub runs: Vec<Run>,
    pub urgent: Vec<Urgent>,
    pub todos: Vec<Todo>,
    pub watches: Vec<(String, SessionRef)>,
    pub sync_state: Vec<(String, String, String)>,
    pub meta: Vec<(String, String)>,
}

fn bad(
    idx: usize,
    e: impl std::error::Error + Send + Sync + 'static,
) -> based_runtime::rusqlite::Error {
    based_runtime::rusqlite::Error::FromSqlConversionFailure(idx, Type::Text, Box::new(e))
}

/// Times as the old store wrote them, e.g. `2026-10-21 08:33:54.145887+00:00`.
pub fn time(text: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S%.f%:z")
        .or_else(|_| DateTime::parse_from_rfc3339(text))
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

fn at(row: &Row<'_>, idx: usize) -> Sql<DateTime<Utc>> {
    let text: String = row.get(idx)?;
    time(&text).ok_or_else(|| bad(idx, std::fmt::Error))
}

fn at_opt(row: &Row<'_>, idx: usize) -> Sql<Option<DateTime<Utc>>> {
    let text: Option<String> = row.get(idx)?;
    text.map(|t| time(&t).ok_or_else(|| bad(idx, std::fmt::Error)))
        .transpose()
}

/// An enum stored by its serde name, e.g. `in_progress`.
fn named<T: DeserializeOwned>(row: &Row<'_>, idx: usize) -> Sql<Option<T>> {
    let text: Option<String> = row.get(idx)?;
    text.map(|t| serde_json::from_value(serde_json::Value::String(t)).map_err(|e| bad(idx, e)))
        .transpose()
}

fn all<T>(conn: &Connection, sql: &str, f: impl FnMut(&Row<'_>) -> Sql<T>) -> AppResult<Vec<T>> {
    let mut stmt = conn.prepare(sql).map_err(legacy_error)?;
    let rows = stmt.query_map([], f).map_err(legacy_error)?;
    rows.collect::<Sql<_>>().map_err(legacy_error)
}

pub fn legacy_error(e: impl std::fmt::Display) -> AppError {
    AppError::Db(format!("reading the old database: {e}"))
}

/// The old status, without "proceeding", which said whose turn it was.
fn status(r: &Row<'_>) -> Sql<Option<Status>> {
    let text: Option<String> = r.get(4)?;
    if matches!(text.as_deref(), Some("proceeding" | "decision")) {
        return Ok(None);
    }
    named(r, 4)
}

/// An old item's turn, from its status and holder: waiting statuses held by someone other than the
/// user or Claude are others' turn.
fn turn(status: Option<Status>, holder: String) -> (Turn, Option<String>) {
    let waits = matches!(
        status,
        Some(Status::Review | Status::Ci | Status::Dependency)
    );
    if waits && !matches!(holder.as_str(), "you" | "Claude") {
        return (Turn::Others, Some(holder));
    }
    (Turn::You, None)
}

fn item(r: &Row<'_>) -> Sql<(WorkItem, bool)> {
    let stage = named(r, 3)?.ok_or_else(|| bad(3, std::fmt::Error))?;
    let status = status(r)?;
    let (turn, waiting_on) = turn(status, r.get(5)?);
    let state_label: String = r.get(2)?;
    let life = Life::from(LifeFields {
        stage,
        turn: Some(turn),
        turn_until: None,
        need: Some(state_label.clone()),
        waiting_on,
        then_turn: None,
        wake: Vec::new(),
    });
    let item = WorkItem {
        id: r.get(0)?,
        title: r.get(1)?,
        state_label,
        life,
        status,
        area: r.get(6)?,
        link: r.get(7)?,
        source: r.get(8)?,
        external_id: r.get(9)?,
        opened_at: at(r, 10)?,
        state_since: at(r, 11)?,
        rank: r.get(12)?,
    };
    Ok((item, r.get(13)?))
}

fn event(r: &Row<'_>) -> Sql<OldEvent> {
    let json: String = r.get(2)?;
    // "proceeding" is retired; old history reads as the status cleared.
    let json = json.replace(r#""status":"proceeding""#, r#""status":null"#);
    let change = serde_json::from_str(&json).map_err(|e| bad(2, e))?;
    Ok((r.get(0)?, at(r, 1)?, change))
}

fn ask(r: &Row<'_>) -> Sql<(Ask, Option<String>)> {
    let options: String = r.get(3)?;
    let session: Option<String> = r.get(4)?;
    let cwd: Option<String> = r.get(5)?;
    let ask = Ask {
        id: 0,
        item_id: r.get(0)?,
        question: r.get(1)?,
        context: r.get(2)?,
        options: serde_json::from_str(&options).unwrap_or_default(),
        asker: session.map(|id| SessionRef { id, cwd }),
    };
    Ok((ask, r.get(6)?))
}

fn commitment(r: &Row<'_>) -> Sql<Commitment> {
    Ok(Commitment {
        id: 0,
        title: r.get(0)?,
        area: r.get(1)?,
        starts_at: at(r, 2)?,
        ends_at: at(r, 3)?,
        location: r.get(4)?,
        join_url: r.get(5)?,
        source: r.get(6)?,
        external_id: r.get(7)?,
        hidden: r.get(8)?,
    })
}

fn activity(r: &Row<'_>) -> Sql<Activity> {
    Ok(Activity {
        id: 0,
        text: r.get(0)?,
        area: r.get(1)?,
        at: at(r, 2)?,
        unread: r.get(3)?,
    })
}

fn run(r: &Row<'_>) -> Sql<Run> {
    Ok(Run {
        id: 0,
        schedule_id: r.get(0)?,
        what: r.get(1)?,
        target: r.get(2)?,
        node: r.get(3)?,
        started_at: at(r, 4)?,
        finished_at: at_opt(r, 5)?,
        outcome: named(r, 6)?,
        summary: r.get(7)?,
    })
}

fn urgent(r: &Row<'_>) -> Sql<Urgent> {
    Ok(Urgent {
        id: 0,
        title: r.get(0)?,
        detail: r.get(1)?,
        area: r.get(2)?,
        raised_at: at(r, 3)?,
        schedule_id: r.get(4)?,
        run_id: None,
    })
}

fn todo(r: &Row<'_>) -> Sql<Todo> {
    let due: Option<String> = r.get(2)?;
    let due = due
        .map(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").map_err(|e| bad(2, e)))
        .transpose()?;
    Ok(Todo {
        id: 0,
        title: r.get(0)?,
        area: r.get(1)?,
        due,
        link: None,
        done_at: at_opt(r, 3)?,
    })
}

fn watch(r: &Row<'_>) -> Sql<(String, SessionRef)> {
    Ok((
        r.get(0)?,
        SessionRef {
            id: r.get(1)?,
            cwd: r.get(2)?,
        },
    ))
}

const ITEMS: &str = "SELECT id, title, state_label, stage, status, holder, area, link, source, \
    external_id, opened_at, state_since, rank, hidden FROM items ORDER BY rank";
const EVENTS: &str = "SELECT item_id, at, change FROM item_events ORDER BY id";
const ASKS: &str =
    "SELECT item_id, question, context, options, session, cwd, answer FROM asks ORDER BY id";
const COMMITMENTS: &str = "SELECT title, area, starts_at, ends_at, location, join_url, source, \
    external_id, hidden FROM commitments ORDER BY id";
const ACTIVITY: &str = "SELECT text, area, at, unread FROM activity ORDER BY id";
const RUNS: &str = "SELECT schedule_id, what, target, node, started_at, finished_at, outcome, \
    summary FROM runs ORDER BY id";
const URGENT: &str = "SELECT title, detail, area, raised_at, schedule_id FROM urgent ORDER BY id";
const TODOS: &str = "SELECT title, area, due, done_at FROM todos ORDER BY id";
const WATCHES: &str = "SELECT item_id, session, cwd FROM watch WHERE session IS NOT NULL";
const SYNC_STATE: &str = "SELECT source, external_id, snapshot FROM sync_state";
const META: &str = "SELECT key, value FROM meta";

pub fn read(path: &Path) -> AppResult<Old> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(legacy_error)?;
    let mut old = Old {
        items: all(&conn, ITEMS, item)?,
        events: all(&conn, EVENTS, event)?,
        asks: all(&conn, ASKS, ask)?,
        watches: all(&conn, WATCHES, watch)?,
        ..Old::default()
    };
    read_records(&conn, &mut old)?;
    Ok(old)
}

fn read_records(conn: &Connection, old: &mut Old) -> AppResult<()> {
    old.commitments = all(conn, COMMITMENTS, commitment)?;
    old.activity = all(conn, ACTIVITY, activity)?;
    old.runs = all(conn, RUNS, run)?;
    old.urgent = all(conn, URGENT, urgent)?;
    old.todos = all(conn, TODOS, todo)?;
    old.sync_state = all(conn, SYNC_STATE, |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
    old.meta = all(conn, META, |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(())
}
