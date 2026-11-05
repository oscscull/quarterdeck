//! Keeps every peer reachable, or loudly not. Each pass checks each peer at its configured URL and,
//! if that fails, at the addresses it last gave (`addresses`); one that answers as the right
//! machine becomes its URL in `quarterdeck.toml`. A peer that answers is told where this machine
//! is. One unreachable for `[sync] down_minutes` raises an urgent card until it's back, since its
//! records, and anything of the user's in them, can't be seen from here meanwhile.

use chrono::{DateTime, Duration, Utc};
use serde_json::json;

use super::{addresses, client};
use crate::config::Config;
use crate::config::edit;
use crate::config::settings::Settings;
use crate::config::sync::Peer;
use crate::error::{AppError, AppResult};
use crate::lasting::{Card, Lasting};
use crate::store::Db;

async fn off_thread<T: Send + 'static>(
    work: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| AppError::Desktop(e.to_string()))?
}

/// Whether `peer` answers as itself.
async fn answers(peer: &Peer) -> bool {
    let p = peer.clone();
    off_thread(move || client::health(&p))
        .await
        .is_ok_and(|h| h.get("node").and_then(|n| n.as_str()) == Some(peer.name.as_str()))
}

/// The peer at a URL that answers, if any: its own first, then where it last said it was.
async fn reach(db: &Db, peer: &Peer) -> AppResult<Option<Peer>> {
    if answers(peer).await {
        return Ok(Some(peer.clone()));
    }
    for url in addresses::learned(db, &peer.name).await? {
        let moved = Peer {
            url,
            ..peer.clone()
        };
        if moved.url != peer.url && answers(&moved).await {
            return Ok(Some(moved));
        }
    }
    Ok(None)
}

/// Tells `peer` where this machine is. A failure only means it learns next time.
async fn announce(peer: Peer, settings: &Settings) {
    let port = settings.sync.port;
    let args = json!({ "from": settings.node, "port": port, "urls": addresses::mine(port) });
    drop(off_thread(move || client::call(&peer, "hello", &args)).await);
}

fn down(peer: &str, after: Duration) -> Lasting {
    Lasting {
        key: format!("peer.{peer}.down"),
        after,
    }
}

async fn unreachable(db: &Db, settings: &Settings, peer: &Peer) -> AppResult<()> {
    let name = peer.name.clone();
    let detail = move |since: DateTime<Utc>| {
        format!(
            "Not answering since {}, at {} or anywhere it last said it was. Its items, questions \
             and to-dos can't be seen from here meanwhile, so check it directly. If its address \
             changed, it tells this machine as soon as it can reach it; otherwise set it with \
             add_peer.",
            since.format("%d %b %H:%M"),
            peer.url
        )
    };
    let title = format!("Can't reach {name}");
    let card = Card {
        title: &title,
        detail: &detail,
        area: "",
    };
    let after = Duration::minutes(settings.sync.down_minutes);
    down(&peer.name, after).holds(db, &card).await.map(drop)
}

/// Checks every peer, following ones that moved and raising ones that are gone.
pub async fn check(db: &Db, config: &Config, settings: &Settings) -> AppResult<()> {
    for peer in &settings.sync.peers {
        let Some(found) = reach(db, peer).await? else {
            unreachable(db, settings, peer).await?;
            continue;
        };
        if found.url != peer.url {
            edit::upsert_peer(config, &found)?;
        }
        down(&peer.name, Duration::zero()).clear(db).await?;
        announce(found, settings).await;
    }
    Ok(())
}
