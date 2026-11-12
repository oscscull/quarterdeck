//! The `[sync]` section of `quarterdeck.toml`: other machines running Quarterdeck.

use serde::{Deserialize, Serialize};

use crate::peers::reach::Listen;

/// Another machine's Quarterdeck, on the local network or through a tunnel the user runs.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Peer {
    /// Its machine name, as used by areas' `node`.
    pub name: String,
    /// e.g. `http://Oscars-MacBook-Pro.local:4791` on the home network, or a tunnel's URL.
    pub url: String,
    /// The `[sync] token` from that machine's quarterdeck.toml.
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SyncSettings {
    /// Serve this machine's Quarterdeck to peers on `port`.
    pub enabled: bool,
    pub port: u16,
    /// Also accept machines on the local network (private addresses only). Off means this machine
    /// only, for use behind a tunnel.
    pub lan: bool,
    /// Peers must send this. Generated on first start when empty.
    pub token: String,
    pub peers: Vec<Peer>,
    /// How long a peer may go unreachable before it's raised as urgent.
    pub down_minutes: i64,
}

impl Default for SyncSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 4791,
            lan: false,
            token: String::new(),
            peers: Vec::new(),
            down_minutes: 30,
        }
    }
}

impl SyncSettings {
    pub const fn listen(&self) -> Listen {
        Listen {
            port: self.port,
            lan: self.lan,
        }
    }

    pub fn peer(&self, name: &str) -> Option<&Peer> {
        self.peers.iter().find(|p| p.name == name)
    }
}
