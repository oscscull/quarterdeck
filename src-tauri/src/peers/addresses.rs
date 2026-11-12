//! Where a peer can be reached. Addresses change (a new network, a VPN, a link-local address that
//! moves), so a peer's configured URL isn't trusted to stay right: each machine tells its peers
//! where it is on every pass (`hello`), the receiver also notes the address the call came from,
//! and those are tried when the configured URL stops answering.

use std::net::SocketAddr;
use std::process::Command;

use crate::cli;
use crate::error::AppResult;
use crate::store::{Db, meta};

fn key(peer: &str) -> String {
    format!("peer.{peer}.addresses")
}

/// A URL for a machine serving on `port` at `ip`. A link-local IPv6 address keeps its zone (as a
/// number, which curl takes), since without one it can't be reached.
pub fn url_at(addr: SocketAddr, port: u16) -> String {
    match addr {
        SocketAddr::V6(v6) if let Some(v4) = v6.ip().to_ipv4_mapped() => {
            format!("http://{v4}:{port}")
        }
        SocketAddr::V6(v6) if v6.scope_id() != 0 => {
            format!("http://[{}%25{}]:{port}", v6.ip(), v6.scope_id())
        }
        SocketAddr::V6(v6) => format!("http://[{}]:{port}", v6.ip()),
        SocketAddr::V4(v4) => format!("http://{}:{port}", v4.ip()),
    }
}

/// URLs this machine can give its peers for itself: its Bonjour name, on a Mac.
pub fn mine(port: u16) -> Vec<String> {
    let name = Command::new(cli::resolve("scutil"))
        .args(["--get", "LocalHostName"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .filter(|n| !n.is_empty());
    name.map(|n| vec![format!("http://{n}.local:{port}")])
        .unwrap_or_default()
}

/// Notes where `peer` says it is, the address it was heard from first.
pub async fn learn(db: &Db, peer: &str, urls: Vec<String>) -> AppResult<()> {
    let mut known: Vec<String> = Vec::new();
    for url in urls {
        if !url.trim().is_empty() && !known.contains(&url) {
            known.push(url);
        }
    }
    meta::set_text(db, &key(peer), &serde_json::to_string(&known)?).await
}

/// Where `peer` last said it was.
pub async fn learned(db: &Db, peer: &str) -> AppResult<Vec<String>> {
    Ok(meta::text(db, &key(peer))
        .await?
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv6Addr, SocketAddrV6};

    use super::*;

    #[test]
    fn urls_reach_the_address_a_call_came_from() {
        let v4: SocketAddr = "192.168.1.20:5555".parse().unwrap();
        assert_eq!(url_at(v4, 4791), "http://192.168.1.20:4791");
        let mapped: SocketAddr = "[::ffff:10.0.0.5]:5555".parse().unwrap();
        assert_eq!(url_at(mapped, 4791), "http://10.0.0.5:4791");
        let ip: Ipv6Addr = "fe80::1".parse().unwrap();
        let link = SocketAddr::V6(SocketAddrV6::new(ip, 5555, 0, 14));
        assert_eq!(url_at(link, 4791), "http://[fe80::1%2514]:4791");
    }

    #[tokio::test]
    async fn learned_addresses_are_kept_once_each_in_order() {
        let db = Db::in_memory().await;
        let urls = ["http://a:1", "http://b:1", "http://a:1", " "].map(str::to_owned);
        learn(&db, "work-mac", urls.to_vec()).await.unwrap();
        assert_eq!(
            learned(&db, "work-mac").await.unwrap(),
            ["http://a:1", "http://b:1"]
        );
    }
}
