//! A scheduler that keeps failing stops every clock, so it can't fail quietly: once ticks have
//! failed for `STALLED_AFTER`, an urgent card says why until a tick works. It needs nothing from
//! the config folder, since a broken config is one way to get here.

use chrono::{DateTime, Duration, Utc};

use crate::error::AppResult;
use crate::lasting::{Card, Lasting};
use crate::store::Db;

/// A blip (a half-written file) passes within a tick or two; longer than this is a stall.
const STALLED_AFTER: Duration = Duration::minutes(5);

fn lasting() -> Lasting {
    Lasting {
        key: "scheduler.failing".to_owned(),
        after: STALLED_AFTER,
    }
}

/// Records how the last tick went, raising or clearing the card.
pub async fn record(db: &Db, outcome: Result<(), String>) -> AppResult<()> {
    let why = match outcome {
        Ok(()) => return lasting().clear(db).await,
        Err(why) => why,
    };
    let detail = |since: DateTime<Utc>| {
        format!(
            "Every tick since {} has failed, so no turn runs out and nothing comes back to you \
             until it's fixed: {why}",
            since.format("%H:%M")
        )
    };
    let card = Card {
        title: "Quarterdeck's clock has stopped",
        detail: &detail,
        area: "",
    };
    lasting().holds(db, &card).await.map(drop)
}
