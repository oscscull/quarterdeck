//! Whether Claude Code is signed in, as `claude auth status` tells it.

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

use crate::cli;

const TIMEOUT: Duration = Duration::from_secs(10);
const SIGNED_OUT: &str = "Claude Code is not logged in: run `claude` in a terminal and use /login";

/// Why the agent can't run for want of a login. `None` when it's signed in, and when `program`
/// can't say (an older Claude Code): a run finds out then.
pub async fn missing(program: &Path) -> Option<String> {
    let mut cmd = Command::new(program);
    if let Some(path) = cli::path_beside(program) {
        cmd.env("PATH", path);
    }
    cmd.args(["auth", "status"])
        .stdin(Stdio::null())
        .kill_on_drop(true);
    let said = tokio::time::timeout(TIMEOUT, cmd.output())
        .await
        .ok()?
        .ok()?;
    signed_out(&said.stdout).then(|| SIGNED_OUT.to_owned())
}

fn signed_out(status: &[u8]) -> bool {
    let logged_in = serde_json::from_slice::<serde_json::Value>(status)
        .ok()
        .and_then(|s| s.get("loggedIn")?.as_bool());
    logged_in == Some(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_clear_no_is_signed_out() {
        assert!(signed_out(br#"{"loggedIn": false, "authMethod": "none"}"#));
        assert!(!signed_out(br#"{"loggedIn": true}"#));
        assert!(!signed_out(b"error: unknown command 'auth'"));
        assert!(!signed_out(b""));
    }
}
