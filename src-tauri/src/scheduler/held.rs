//! The session holding an item, when Claude can still resume it. One it can't (its transcript is
//! gone, or the id was never real) is let go with a note on the item, so work carries on in a new
//! session instead of failing on it every time.

use chrono::Utc;

use crate::claude::sessions;
use crate::domain::event::Change;
use crate::domain::session::SessionRef;
use crate::error::AppResult;
use crate::store::{Db, events, watch};

pub async fn session(db: &Db, item_id: &str) -> AppResult<Option<SessionRef>> {
    let Some(held) = watch::session_for(db, item_id).await? else {
        return Ok(None);
    };
    if sessions::exists(&held) {
        return Ok(Some(held));
    }
    watch::release(db, item_id).await?;
    let text = format!(
        "Let go of session {}: Claude can't resume it here, so work carries on in a new one",
        held.id
    );
    events::record(db, item_id, Utc::now(), &Change::Note { text }).await?;
    Ok(None)
}
