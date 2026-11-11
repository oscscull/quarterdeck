//! Whether Claude Code is signed in, as `claude auth status` tells it.

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

use super::Program;
use crate::cli;

const TIMEOUT: Duration = Duration::from_secs(10);

/// Why the agent can't run for want of a login, asked as the account runs use. `None` when it's
/// signed in, and when `program` can't say (an older Claude Code): a run finds out then.
pub async fn missing(program: &Path, claude: &Program) -> Option<String> {
    let mut cmd = Command::new(program);
    claude.as_user(&mut cmd);
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
    signed_out(&said.stdout).then(|| signed_out_as(&claude.user))
}

fn signed_out_as(user: &str) -> String {
    format!(
        "Claude Code is not logged in as `{user}`: log in as that account (`claude`, then /login), \
         or set claude_user in quarterdeck.toml to the account that is"
    )
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
    fn signed_out_names_the_account_and_the_setting() {
        let why = signed_out_as("oscar");
        assert!(why.contains("`oscar`") && why.contains("claude_user"));
        assert!(super::super::reach::unreached(&why));
    }

    #[test]
    fn only_a_clear_no_is_signed_out() {
        assert!(signed_out(br#"{"loggedIn": false, "authMethod": "none"}"#));
        assert!(!signed_out(br#"{"loggedIn": true}"#));
        assert!(!signed_out(b"error: unknown command 'auth'"));
        assert!(!signed_out(b""));
    }
}
