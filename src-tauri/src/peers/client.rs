//! Calls a tool on a peer's Quarterdeck. Blocking: run it off the UI thread.

use std::io::Write as _;
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::cli;
use crate::config::sync::Peer;
use crate::error::{AppError, AppResult};

const TIMEOUT_SECS: &str = "10";
/// Tools that run Claude on the peer, so take minutes rather than seconds.
const SLOW_TOOLS: [&str; 1] = ["correct_briefing"];
const SLOW_TIMEOUT_SECS: &str = "300";

fn url(peer: &Peer, path: &str) -> String {
    format!("{}/v1/{path}", peer.url.trim_end_matches('/'))
}

fn request(peer: &Peer, path: &str, body: Option<&Value>, timeout: &str) -> AppResult<Value> {
    let auth = format!("Authorization: Bearer {}", peer.token);
    let mut cmd = Command::new(cli::resolve("curl"));
    cmd.args([
        "-sS",
        "--max-time",
        timeout,
        "-H",
        &auth,
        "-H",
        "Content-Type: application/json",
    ]);
    if body.is_some() {
        cmd.args(["-X", "POST", "--data-binary", "@-"]);
    }
    let mut child = cmd
        .arg(url(peer, path))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let (Some(body), Some(stdin)) = (body, child.stdin.as_mut()) {
        stdin.write_all(serde_json::to_string(body)?.as_bytes())?;
    }
    drop(child.stdin.take());
    let out = child.wait_with_output()?;
    if !out.status.success() {
        let why = String::from_utf8_lossy(&out.stderr).trim().to_owned();
        return Err(AppError::Connector(format!(
            "{} is unreachable: {why}",
            peer.name
        )));
    }
    let reply: Value = serde_json::from_slice(&out.stdout)?;
    if let Some(error) = reply.get("error").and_then(Value::as_str) {
        return Err(AppError::Connector(format!("{}: {error}", peer.name)));
    }
    Ok(reply.get("ok").cloned().unwrap_or(Value::Null))
}

/// Runs one Quarterdeck tool on the peer and returns its result.
pub fn call(peer: &Peer, tool: &str, args: &Value) -> AppResult<Value> {
    let timeout = if SLOW_TOOLS.contains(&tool) {
        SLOW_TIMEOUT_SECS
    } else {
        TIMEOUT_SECS
    };
    request(peer, &format!("tools/{tool}"), Some(args), timeout)
}

/// The peer's machine name, if it answers.
pub fn health(peer: &Peer) -> AppResult<Value> {
    request(peer, "health", None, TIMEOUT_SECS)
}
