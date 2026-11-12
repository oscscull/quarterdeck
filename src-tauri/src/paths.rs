//! Where Quarterdeck keeps things. Shared by the app and the MCP server so both see the same data.

use std::path::PathBuf;

use crate::error::{AppError, AppResult};
use crate::profile;

pub fn home() -> AppResult<PathBuf> {
    dirs::home_dir().ok_or_else(|| AppError::NotFound("the home folder".to_owned()))
}

/// A path as the user writes it: `~` and `~/…` are the home folder.
pub fn expand_home(path: &str) -> AppResult<PathBuf> {
    Ok(match path {
        "~" => home()?,
        _ => match path.strip_prefix("~/") {
            Some(rest) => home()?.join(rest),
            None => PathBuf::from(path),
        },
    })
}

/// The database lives here: the system's app-data folder, as Tauri uses (on macOS
/// `~/Library/Application Support/<id>`, on Windows `%APPDATA%\<id>`, on Linux `~/.local/share/<id>`).
pub fn data_dir() -> AppResult<PathBuf> {
    data_dir_of(profile::APP_ID)
}

/// The installed app's data folder, which a dev build copies from (`seed-dev`).
pub fn installed_data_dir() -> AppResult<PathBuf> {
    data_dir_of(profile::INSTALLED_ID)
}

fn data_dir_of(app_id: &str) -> AppResult<PathBuf> {
    let base =
        dirs::data_dir().ok_or_else(|| AppError::NotFound("the app-data folder".to_owned()))?;
    Ok(base.join(app_id))
}

/// Behaviour the user and agents edit: settings, prompts, schedules, inbox.
pub fn config_dir() -> AppResult<PathBuf> {
    Ok(home()?.join(".config").join(profile::CONFIG))
}

pub fn installed_config_dir() -> AppResult<PathBuf> {
    Ok(home()?.join(".config").join(profile::INSTALLED_CONFIG))
}

/// Claude Code's user config, where `claude mcp add --scope user` registers servers.
pub fn claude_config() -> AppResult<PathBuf> {
    Ok(home()?.join(".claude.json"))
}
