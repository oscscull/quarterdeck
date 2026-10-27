//! The user's config folder: settings, prompts and schedules as plain files.
//! Users and agents edit these directly; the app re-reads them on each request.

mod defaults;
mod frontmatter;

pub mod alerts;
pub mod branches;
pub mod calendar;
pub mod dev;
pub mod edit;
pub mod github;
pub mod inbox;
pub mod issues;
pub mod keys;
pub mod outage;
pub mod pause;
pub mod projects;
pub mod prompts;
pub mod queue;
pub mod schedules;
pub mod settings;
pub mod sync;
pub mod turns;
pub mod updates;

use std::path::{Path, PathBuf};

use crate::error::AppResult;

/// Location of the config folder, e.g. `~/.config/quarterdeck`.
#[derive(Debug, Clone)]
pub struct Config {
    root: PathBuf,
}

impl Config {
    /// Opens the folder, writing any default file that is missing.
    pub fn open(root: PathBuf) -> AppResult<Self> {
        defaults::write_missing(&root)?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}
