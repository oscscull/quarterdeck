//! Runs headless Claude Code (`claude -p`) on the user's own subscription.

pub mod connect;
pub mod draft;
mod follow;
pub mod reach;
pub mod runner;
mod stream;
pub mod transcript;

use tokio::process::Command;

use crate::cli;
use crate::error::AppResult;
use crate::store::fired::DEPTH_VAR;

/// Lets a headless run use every Quarterdeck tool without asking.
const QUARTERDECK_TOOLS: &str = "mcp__quarterdeck";

/// The Quarterdeck MCP server for a headless run: this same binary, started with `mcp`. Events
/// the run causes are `depth` steps down the chain that started it.
fn quarterdeck_server(depth: u32) -> AppResult<String> {
    let exe = std::env::current_exe()?;
    let config = serde_json::json!({
        "mcpServers": { "quarterdeck": {
            "command": exe, "args": ["mcp"], "env": { DEPTH_VAR: depth.to_string() }
        } }
    });
    Ok(config.to_string())
}

/// How to run one headless Claude call.
#[derive(Debug, Clone, Copy, Default)]
pub struct Ask<'a> {
    /// Claude Code tools it may use without asking, besides the Quarterdeck tools.
    pub tools: &'a [String],
    pub session: Session<'a>,
    /// The folder to run in. A resumed session must run where it started.
    pub cwd: Option<&'a str>,
    /// Claude Code's `--permission-mode`, e.g. `acceptEdits`. Commands still need the project's
    /// own permission rules.
    pub permission_mode: Option<&'a str>,
    /// The run record whose steps to log for the Agent tab, if it has one.
    pub run: Option<i64>,
    /// How many events led to this run; 0 for anything not started by one.
    pub depth: u32,
}

/// Which Claude Code session a run belongs to.
#[derive(Debug, Clone, Copy, Default)]
pub enum Session<'a> {
    /// A throwaway session.
    #[default]
    Fresh,
    /// A new session with this id, so Quarterdeck can come back to it later.
    Start(&'a str),
    /// Continue an existing session with its own context.
    Resume(&'a str),
}

/// Sends one prompt and returns Claude's reply text. The Quarterdeck tools are always available.
pub async fn ask_with(command: &str, prompt: &str, opts: &Ask<'_>) -> AppResult<String> {
    let mut allowed = vec![QUARTERDECK_TOOLS.to_owned()];
    allowed.extend_from_slice(opts.tools);
    let program = cli::resolve(command);
    let mut cmd = Command::new(&program);
    if let Some(path) = cli::path_beside(&program) {
        cmd.env("PATH", path);
    }
    cmd.arg("-p")
        .arg(prompt)
        .arg("--mcp-config")
        .arg(quarterdeck_server(opts.depth)?)
        .arg("--allowedTools")
        .arg(allowed.join(","));
    if let Some(dir) = opts.cwd {
        cmd.current_dir(dir);
    }
    if let Some(mode) = opts.permission_mode {
        cmd.arg("--permission-mode").arg(mode);
    }
    match opts.session {
        Session::Fresh => {}
        Session::Start(id) => drop(cmd.arg("--session-id").arg(id)),
        Session::Resume(id) => drop(cmd.arg("--resume").arg(id)),
    }
    follow::follow(cmd, follow::open_log(opts.run)).await
}

/// Sends one prompt with only the Quarterdeck tools.
pub async fn ask(command: &str, prompt: &str) -> AppResult<String> {
    ask_with(command, prompt, &Ask::default()).await
}
