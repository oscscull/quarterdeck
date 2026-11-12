//! Agents raise urgent items by dropping a file in `inbox/`, e.g. `inbox/ci-red.toml`:
//!
//! ```toml
//! title = "CI on main is red"
//! detail = "Blocks 3 PRs."
//! ```
//!
//! Quarterdeck takes each file once and deletes it, so the folder never fills up.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::Config;
use crate::domain::urgent::Urgent;
use crate::error::AppResult;

/// An urgent item an agent asked for.
#[derive(Debug, Clone, Deserialize)]
pub struct Raised {
    pub title: String,
    #[serde(default)]
    pub detail: String,
    /// Checked against the configured areas by the caller.
    #[serde(default)]
    pub area: String,
    pub schedule_id: Option<String>,
}

impl Raised {
    pub fn into_urgent(self, area: String, at: DateTime<Utc>) -> Urgent {
        Urgent {
            id: 0,
            title: self.title,
            detail: self.detail,
            area,
            raised_at: at,
            schedule_id: self.schedule_id,
            run_id: None,
        }
    }
}

fn dir(config: &Config) -> PathBuf {
    config.root().join("inbox")
}

/// Reads and removes every inbox file. A file that can't be read becomes an item saying so.
pub fn take(config: &Config) -> AppResult<Vec<Raised>> {
    let dir = dir(config);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut raised = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let file = entry?.path();
        if file.extension().is_some_and(|e| e == "toml") {
            raised.push(read(&file));
            std::fs::remove_file(&file)?;
        }
    }
    Ok(raised)
}

fn read(file: &Path) -> Raised {
    std::fs::read_to_string(file)
        .map_err(|e| e.to_string())
        .and_then(|text| toml::from_str(&text).map_err(|e| e.to_string()))
        .unwrap_or_else(|reason| Raised {
            title: format!("Couldn't read {}", file.display()),
            detail: reason,
            area: String::new(),
            schedule_id: None,
        })
}
