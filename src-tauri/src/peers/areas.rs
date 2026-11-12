//! Every machine keeps the same `[[areas]]` list, since each area's `node` decides where its
//! schedules and connectors run. Per peer, this machine remembers the list the two last agreed on:
//! when its own list differs from that (an edit by a tool or by hand), it's dirty for that peer and
//! is pushed on the next tick, until the peer takes it. A peer only takes a list if its own hasn't
//! changed since they last agreed; if both changed, the user is asked to pick.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::client;
use crate::config::settings::{self, Area, Settings};
use crate::config::sync::Peer;
use crate::config::{Config, edit};
use crate::domain::urgent::Urgent;
use crate::error::{AppError, AppResult};
use crate::store::{Db, meta, urgent};

/// The list both machines last had, as [`canonical`] text. Followed by the peer's name.
const AGREED: &str = "areas.agreed.";
/// The list a conflict was last raised for, so it's raised once, not every tick.
const CONFLICT: &str = "areas.conflict.";

/// What a peer did with a list it was sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// It already had this list.
    Same,
    /// It replaced its own list with this one.
    Taken,
    /// Both lists changed since they last agreed.
    Conflict,
}

/// The list in a form two machines can compare: sorted by id.
pub fn canonical(areas: &[Area]) -> AppResult<String> {
    let mut sorted = areas.to_vec();
    sorted.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(serde_json::to_string(&sorted)?)
}

/// Whether this machine and `peer` have the same areas, as far as this machine knows.
pub async fn agreed(db: &Db, settings: &Settings, peer: &str) -> AppResult<bool> {
    let last = meta::text(db, &format!("{AGREED}{peer}")).await?;
    Ok(last == Some(canonical(&settings.areas)?))
}

async fn record(db: &Db, peer: &str, list: &str) -> AppResult<()> {
    meta::set_text(db, &format!("{AGREED}{peer}"), list).await?;
    meta::forget(db, &format!("{CONFLICT}{peer}")).await
}

/// A peer's list arrives. Taken only if ours hasn't changed since we last agreed with it.
pub async fn receive(db: &Db, config: &Config, from: &str, areas: &[Area]) -> AppResult<Outcome> {
    let mine = canonical(&settings::load(config)?.areas)?;
    let theirs = canonical(areas)?;
    if mine == theirs {
        record(db, from, &mine).await?;
        return Ok(Outcome::Same);
    }
    let last = meta::text(db, &format!("{AGREED}{from}")).await?;
    if last.as_deref() != Some(mine.as_str()) {
        return Ok(Outcome::Conflict);
    }
    edit::set_areas(config, areas)?;
    record(db, from, &theirs).await?;
    Ok(Outcome::Taken)
}

async fn send(peer: Peer, args: Value) -> AppResult<Outcome> {
    let reply =
        tauri::async_runtime::spawn_blocking(move || client::call(&peer, "receive_areas", &args))
            .await
            .map_err(|e| AppError::Desktop(e.to_string()))??;
    Ok(serde_json::from_value(reply)?)
}

/// Raises "areas differ" once per conflicting list.
async fn raise_conflict(db: &Db, settings: &Settings, peer: &str, mine: &str) -> AppResult<()> {
    let key = format!("{CONFLICT}{peer}");
    if meta::text(db, &key).await?.as_deref() == Some(mine) {
        return Ok(());
    }
    let item = Urgent {
        id: 0,
        title: format!("Areas differ between {} and {peer}", settings.node),
        detail: format!(
            "Both machines changed [[areas]] since they last matched. Edit one machine's list to match \
             the other's; until then schedules don't move between {} and {peer}.",
            settings.node
        ),
        area: settings.known_area(""),
        raised_at: chrono::Utc::now(),
        schedule_id: None,
        run_id: None,
    };
    urgent::raise(db, &item).await?;
    meta::set_text(db, &key, mine).await
}

/// Sends this machine's areas to every peer it doesn't agree with. A peer that can't be reached
/// stays dirty and is tried again next time.
pub async fn push(db: &Db, settings: &Settings) -> AppResult<()> {
    let mine = canonical(&settings.areas)?;
    let args = json!({ "from": settings.node, "areas": settings.areas });
    for peer in &settings.sync.peers {
        if agreed(db, settings, &peer.name).await? {
            continue;
        }
        match send(peer.clone(), args.clone()).await {
            Ok(Outcome::Same | Outcome::Taken) => record(db, &peer.name, &mine).await?,
            Ok(Outcome::Conflict) => raise_conflict(db, settings, &peer.name, &mine).await?,
            Err(_) => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(id: &str, node: &str) -> Area {
        Area {
            id: id.to_owned(),
            name: id.to_owned(),
            color: "#000".to_owned(),
            node: node.to_owned(),
        }
    }

    async fn check(ctx: &crate::mcp::Ctx, list: &[Area]) -> Outcome {
        receive(&ctx.db, &ctx.config, "peer", list).await.unwrap()
    }

    #[test]
    fn order_doesnt_matter() {
        let a = [area("work", "w"), area("life", "h")];
        let b = [area("life", "h"), area("work", "w")];
        assert_eq!(canonical(&a).unwrap(), canonical(&b).unwrap());
    }

    #[test]
    fn a_peer_takes_a_list_only_while_its_own_is_unchanged() {
        let dir = std::env::temp_dir().join(format!("quarterdeck-areas-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        tauri::async_runtime::block_on(async {
            let ctx = crate::mcp::Ctx::at(&dir.join("data"), dir.join("config"))
                .await
                .unwrap();
            let mut list = settings::load(&ctx.config).unwrap().areas;
            // Never agreed and different: both must have changed.
            list[0].node = "elsewhere".to_owned();
            assert_eq!(check(&ctx, &list).await, Outcome::Conflict);
            // The same list is agreed on.
            let same = settings::load(&ctx.config).unwrap().areas;
            assert_eq!(check(&ctx, &same).await, Outcome::Same);
            // Now unchanged here, so the peer's change is taken.
            assert_eq!(check(&ctx, &list).await, Outcome::Taken);
            let now = settings::load(&ctx.config).unwrap().areas;
            assert_eq!(canonical(&now).unwrap(), canonical(&list).unwrap());
        });
    }
}
