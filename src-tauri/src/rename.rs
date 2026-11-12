//! Moves an install from the app's old name (Bridge, app id `com.oscscull.dayscheduler`) to
//! Quarterdeck, once, before anything opens the config or database. Delete this file once every
//! machine has started a Quarterdeck build.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::claude::sessions;
use crate::cli;
use crate::error::AppResult;
use crate::paths;

const OLD_DATA: &str = "Library/Application Support/com.oscscull.dayscheduler";
const OLD_CONFIG: &str = ".config/bridge";

/// Renames `from` to `to` when `from` exists and `to` doesn't.
fn move_once(from: &Path, to: &Path) -> AppResult<bool> {
    if !from.exists() || to.exists() {
        return Ok(false);
    }
    std::fs::rename(from, to)?;
    Ok(true)
}

pub fn migrate() -> AppResult<()> {
    let home = paths::home()?;
    move_config(&home.join(OLD_CONFIG), &paths::config_dir()?)?;
    let (old, new) = (home.join(OLD_DATA), paths::data_dir()?);
    if move_once(&old, &new)? {
        move_database(&new)?;
        move_worktrees(&home, &old, &new)?;
    }
    Ok(())
}

fn move_config(old: &Path, new: &Path) -> AppResult<()> {
    move_once(old, new)?;
    move_once(&new.join("bridge.toml"), &new.join("quarterdeck.toml"))?;
    let ledger = new.join(".bridge-defaults");
    if let Ok(text) = std::fs::read_to_string(&ledger) {
        let text = text.replace("bridge.toml ", "quarterdeck.toml ");
        std::fs::write(new.join(".quarterdeck-defaults"), text)?;
        std::fs::remove_file(ledger)?;
    }
    Ok(())
}

fn move_database(dir: &Path) -> AppResult<()> {
    for suffix in ["", "-wal", "-shm"] {
        let name = |stem: &str| dir.join(format!("{stem}.sqlite{suffix}"));
        move_once(&name("bridge"), &name("quarterdeck"))?;
    }
    Ok(())
}

fn session_folder(home: &Path, dir: &Path) -> PathBuf {
    sessions::folder(&home.join(".claude"), dir)
}

/// Worktrees moved with the data folder: tell git, and
/// move Claude's session history so those sessions can still be resumed.
fn move_worktrees(home: &Path, old: &Path, new: &Path) -> AppResult<()> {
    let Ok(entries) = std::fs::read_dir(new.join("worktrees")) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let moved = entry.path();
        // Best effort: a worktree git can't repair is swept and remade when next needed.
        drop(
            Command::new(cli::resolve("git"))
                .arg("-C")
                .arg(&moved)
                .args(["worktree", "repair"])
                .status(),
        );
        let was = old.join("worktrees").join(entry.file_name());
        move_once(&session_folder(home, &was), &session_folder(home, &moved))?;
    }
    Ok(())
}
