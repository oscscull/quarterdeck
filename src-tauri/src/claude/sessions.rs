//! Claude Code sessions on this machine: whether one can be resumed, and which one is calling the MCP
//! server. Claude only resumes a session from the folder it started in, where it keeps its
//! transcript, so a session Quarterdeck remembers must have one there.

use std::path::{Path, PathBuf};

use crate::domain::session::SessionRef;
use crate::error::{AppError, AppResult};
use crate::paths;

/// Set by Claude Code for everything it starts, the MCP server included.
const SESSION_VAR: &str = "CLAUDE_CODE_SESSION_ID";
/// Where Claude Code keeps its state, when not `~/.claude`.
const CONFIG_VAR: &str = "CLAUDE_CONFIG_DIR";

/// Claude Code keeps a folder's sessions under its path with every non-alphanumeric character as `-`.
pub fn folder(claude: &Path, dir: &Path) -> PathBuf {
    let key: String = dir
        .display()
        .to_string()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    claude.join("projects").join(key)
}

fn claude_dir() -> AppResult<PathBuf> {
    match std::env::var_os(CONFIG_VAR) {
        Some(dir) if !dir.is_empty() => Ok(PathBuf::from(dir)),
        _ => Ok(paths::home()?.join(".claude")),
    }
}

/// The folder a session runs in: its own, or this process's for one with none.
fn dir_of(session: &SessionRef) -> AppResult<PathBuf> {
    match &session.cwd {
        Some(cwd) => Ok(PathBuf::from(cwd)),
        None => Ok(std::env::current_dir()?),
    }
}

fn transcript(claude: &Path, session: &SessionRef) -> AppResult<PathBuf> {
    let file = format!("{}.jsonl", session.id.trim());
    Ok(folder(claude, &dir_of(session)?).join(file))
}

/// Whether Claude can resume the session here.
pub fn exists(session: &SessionRef) -> bool {
    !session.id.trim().is_empty()
        && claude_dir()
            .and_then(|c| transcript(&c, session))
            .is_ok_and(|t| t.is_file())
}

/// The session calling the MCP server, from Claude Code's environment, in the folder it started in.
fn calling() -> Option<SessionRef> {
    let id = std::env::var(SESSION_VAR)
        .ok()
        .filter(|i| !i.trim().is_empty())?;
    let cwd = std::env::current_dir().ok()?.display().to_string();
    Some(SessionRef { id, cwd: Some(cwd) })
}

/// A session passed to a tool, with this process's folder when it gave none, since Claude Code
/// starts the MCP server in the session's own.
fn with_folder(passed: SessionRef) -> SessionRef {
    let cwd = passed.cwd.or_else(|| {
        std::env::current_dir()
            .ok()
            .map(|d| d.display().to_string())
    });
    SessionRef { cwd, ..passed }
}

/// The session to remember for a tool call: the one passed if Claude can resume it, else the
/// caller's own. One passed that can't be resumed is refused, naming the caller's, so a guessed id
/// is never kept to fail later.
pub fn resolve(passed: Option<SessionRef>) -> AppResult<Option<SessionRef>> {
    let own = calling().filter(exists);
    let Some(passed) = passed.map(with_folder) else {
        return Ok(own);
    };
    if exists(&passed) {
        return Ok(Some(passed));
    }
    let hint = own.map_or_else(
        || "leave `session` out".to_owned(),
        |s| format!("yours is {}, or leave `session` out to use it", s.id),
    );
    Err(AppError::BadInput(format!(
        "Claude Code has no session {} in {}, so it couldn't be resumed: {hint}",
        passed.id,
        passed.cwd.unwrap_or_default()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sessions_live_under_their_folder() {
        let claude = Path::new("/h/.claude");
        assert_eq!(
            folder(claude, Path::new("/Users/me/my app.v2")),
            Path::new("/h/.claude/projects/-Users-me-my-app-v2")
        );
    }

    #[test]
    fn a_session_is_resumable_only_with_its_transcript() {
        let claude = std::env::temp_dir().join(format!("qd-sessions-{}", uuid::Uuid::new_v4()));
        let session = SessionRef {
            id: "abc".to_owned(),
            cwd: Some("/work/app".to_owned()),
        };
        let path = transcript(&claude, &session).unwrap();
        assert!(!path.is_file());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "{}").unwrap();
        assert!(path.ends_with("projects/-work-app/abc.jsonl"));
        assert!(path.is_file());
        std::fs::remove_dir_all(&claude).unwrap();
    }
}
