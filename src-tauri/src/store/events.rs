use chrono::{DateTime, Utc};

use serde::Serialize;

use super::client::{self, EventRow};
use super::convert::{int, text_key};
use super::db::Db;
use super::{fired, items, time};
use crate::domain::event::{Change, ItemEvent};
use crate::domain::fired::Fired;
use crate::error::AppResult;

fn from_row(row: EventRow) -> AppResult<ItemEvent> {
    Ok(ItemEvent {
        id: int(&row.id)?,
        item_id: row.item_id.into_raw(),
        at: time::get(&row.at)?,
        change: serde_json::from_value(row.change)?,
    })
}

/// An item's history, newest first.
pub async fn for_item(db: &Db, item_id: &str) -> AppResult<Vec<ItemEvent>> {
    let input = client::ItemEventsInput {
        item: text_key(item_id),
    };
    let rows = db.api().item_events(input, ()).await?;
    rows.into_iter().map(from_row).collect()
}

/// Adds a line to an item's history and fires the event it stands for, if any.
pub async fn record(db: &Db, item_id: &str, at: DateTime<Utc>, change: &Change) -> AppResult<()> {
    restore(db, item_id, at, change).await?;
    announce(db, item_id, change).await
}

/// Adds a line to an item's history without firing anything, e.g. when copying old history.
pub async fn restore(db: &Db, item_id: &str, at: DateTime<Utc>, change: &Change) -> AppResult<()> {
    let input = client::RecordEventInput {
        item: text_key(item_id),
        at: time::put(at),
        change: serde_json::to_value(change)?,
    };
    Ok(db.api().record_event(input, ()).await?)
}

/// A value's serde name, e.g. `in_review`.
fn name(value: &impl Serialize) -> AppResult<String> {
    Ok(serde_json::to_value(value)?
        .as_str()
        .unwrap_or_default()
        .to_owned())
}

/// An event's kind, and the field it adds besides `item`.
type ItemEventKind = (&'static str, Option<(&'static str, String)>);

/// The event a change fires, if any.
fn event_for(change: &Change) -> AppResult<Option<ItemEventKind>> {
    Ok(match change {
        Change::Created => Some(("item.created", None)),
        Change::Stage { stage } => Some(("item.stage", Some(("stage", name(stage)?)))),
        Change::Turn { turn, .. } => Some(("item.turn", Some(("to", name(turn)?)))),
        Change::Status { .. } | Change::Label { .. } | Change::Note { .. } => None,
    })
}

async fn announce(db: &Db, item_id: &str, change: &Change) -> AppResult<()> {
    let Some((kind, field)) = event_for(change)? else {
        return Ok(());
    };
    let item = items::get(db, item_id).await?;
    let mut event = Fired::new(kind, &item.area, item_id, String::new()).with("item", item_id);
    let mut detail = String::new();
    if let Some((field, value)) = field {
        detail = format!(": {field} {value}");
        event = event.with(field, &value);
    }
    let why = match change {
        Change::Turn { why, .. } => format!(" ({why})"),
        Change::Created
        | Change::Stage { .. }
        | Change::Status { .. }
        | Change::Label { .. }
        | Change::Note { .. } => String::new(),
    };
    event.about = format!("{kind} on \"{}\" ({item_id}){detail}{why}", item.title);
    fired::emit(db, &event).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::item::{ItemInput, Stage};
    use crate::domain::turn::Turn;

    #[tokio::test]
    async fn changes_fire_item_events_but_restored_history_does_not() {
        let db = Db::in_memory().await;
        let input = ItemInput {
            id: "B-1".to_owned(),
            title: "Thing".to_owned(),
            state_label: String::new(),
            stage: Stage::UpNext,
            status: None,
            area: "work".to_owned(),
            link: None,
        };
        let id = crate::work::save(&db, input, Utc::now()).await.unwrap();
        let turn = Change::Turn {
            turn: Turn::Agent,
            why: "over to Claude".to_owned(),
        };
        record(&db, &id, Utc::now(), &turn).await.unwrap();
        restore(&db, &id, Utc::now(), &turn).await.unwrap();
        let kinds: Vec<_> = fired::new_events(&db, 10)
            .await
            .unwrap()
            .into_iter()
            .map(|e| (e.kind, e.area, e.data.get("to").cloned()))
            .collect();
        let turned = (
            "item.turn".to_owned(),
            "work".to_owned(),
            Some("agent".to_owned()),
        );
        assert!(kinds.iter().any(|k| k.0 == "item.created"));
        assert_eq!(kinds.iter().filter(|k| **k == turned).count(), 1);
    }
}
