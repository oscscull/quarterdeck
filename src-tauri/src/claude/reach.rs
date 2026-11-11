//! Whether the agent can be reached at all: a quick check before prompting it, and telling a run
//! that never reached it (offline, `claude` missing or logged out, GitHub unreachable from `gh` or
//! `git`) from one that went wrong.

use tokio::net::TcpStream;

use super::{Program, login};
use crate::cli;
use crate::config::outage::OutageSettings;

/// Lower-case pieces of the errors a run gives when it couldn't reach the agent, from the
/// `claude_command` hint in `follow`, the network errors Claude Code and Node report, those `gh`
/// and `git` give while making a worktree or pushing, and what Claude Code says when logged out.
const UNREACHED: &[&str] = &[
    "set claude_command",
    "connection error",
    "connection refused",
    "could not resolve host",
    "econnrefused",
    "econnreset",
    "enotfound",
    "etimedout",
    "eai_again",
    "getaddrinfo",
    "fetch failed",
    "network is unreachable",
    "request timed out",
    "overloaded",
    "error connecting to",
    "failed to connect to",
    "could not resolve hostname",
    "connection timed out",
    "operation timed out",
    "temporary failure in name resolution",
    "not logged in",
    "run /login",
    "invalid api key",
    "authentication_error",
    "oauth token",
];

/// Whether a run failed because it couldn't reach the agent, rather than going wrong.
pub fn unreached(detail: &str) -> bool {
    let detail = detail.to_lowercase();
    UNREACHED.iter().any(|p| detail.contains(p))
}

/// Why the agent can't run now, or `None` when it can: its command must exist, each probe answer
/// and Claude Code be logged in as the account runs use.
pub async fn check(claude: &Program, settings: &OutageSettings) -> Option<String> {
    let command = &claude.command;
    // Finding it may ask the login shell, which blocks.
    let owned = command.clone();
    let found = tokio::task::spawn_blocking(move || {
        let program = cli::resolve(&owned);
        program.exists().then_some(program)
    })
    .await;
    let Some(program) = found.ok().flatten() else {
        return Some(format!(
            "`{command}` wasn't found; set claude_command in quarterdeck.toml to its full path"
        ));
    };
    for probe in [&settings.probe, &settings.github_probe] {
        if let Some(why) = unanswered(probe, settings).await {
            return Some(why);
        }
    }
    login::missing(&program, claude).await
}

/// Why `probe` (`host:port`) didn't answer; `None` when it did or is empty.
async fn unanswered(probe: &str, settings: &OutageSettings) -> Option<String> {
    if probe.is_empty() {
        return None;
    }
    match tokio::time::timeout(settings.timeout(), TcpStream::connect(probe)).await {
        Ok(Ok(_)) => None,
        Ok(Err(e)) => Some(format!("offline: can't reach {probe} ({e})")),
        Err(_) => Some(format!("offline: {probe} didn't answer")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_and_missing_program_errors_are_unreached() {
        assert!(unreached(
            "set claude_command in quarterdeck.toml to its full path: `claude` wasn't found"
        ));
        assert!(unreached("API Error: Connection error."));
        assert!(unreached("getaddrinfo ENOTFOUND api.anthropic.com"));
        assert!(unreached("API Error: 529 {\"type\":\"overloaded_error\"}"));
        assert!(unreached(
            "gh pr view 7 --repo me/app: error connecting to api.github.com"
        ));
        assert!(unreached(
            "git push: fatal: unable to access 'https://github.com/me/app/': Failed to connect to github.com port 443"
        ));
    }

    #[test]
    fn a_logged_out_agent_is_unreached() {
        assert!(unreached(
            "claude failed: Not logged in · Please run /login"
        ));
        assert!(unreached("Invalid API key · Please run /login"));
        assert!(unreached(
            "API Error: 401 {\"type\":\"authentication_error\"}"
        ));
    }

    #[test]
    fn a_run_that_went_wrong_is_not_unreached() {
        assert!(!unreached("it exited with no message (exit status: 1)"));
        assert!(!unreached(
            "send it again to start afresh: the folder this run belonged in is gone"
        ));
    }

    fn program(command: &str) -> Program {
        Program {
            command: command.to_owned(),
            model: String::new(),
            user: "oscar".to_owned(),
        }
    }

    #[tokio::test]
    async fn a_missing_program_fails_the_check() {
        let why = check(&program("/nowhere/claude"), &OutageSettings::default()).await;
        assert!(why.is_some_and(|w| w.contains("wasn't found")));
    }

    #[tokio::test]
    async fn no_probe_checks_only_the_program() {
        let settings = OutageSettings {
            probe: String::new(),
            github_probe: String::new(),
            ..OutageSettings::default()
        };
        assert_eq!(check(&program("/bin/sh"), &settings).await, None);
    }
}
