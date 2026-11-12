//! Runs a script in the user's login shell, streaming its output into the run's log, with a time
//! limit.

use std::path::PathBuf;
use std::process::{ExitStatus, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use chrono::Utc;
use tokio::io::{AsyncBufReadExt as _, AsyncRead, BufReader};
use tokio::process::{Child, Command};

use crate::domain::step::{Step, StepKind};
use crate::error::AppResult;
use crate::runlog::Writer;

/// Characters of each output line kept in the log.
const LINE_CHARS: usize = 300;

/// What to run, where, and for how long.
#[derive(Debug)]
pub struct Shell<'a> {
    pub script: &'a str,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
    pub limit: Duration,
}

/// How it ended: whether it succeeded, and one line about it.
#[derive(Debug, PartialEq, Eq)]
pub struct Exit {
    pub ok: bool,
    pub line: String,
}

#[cfg(windows)]
fn command(script: &str) -> Command {
    let mut cmd = Command::new("cmd");
    cmd.arg("/C").arg(script);
    cmd
}

/// A login shell, so the user's PATH (Homebrew, nvm…) is there as in their terminal.
#[cfg(not(windows))]
fn command(script: &str) -> Command {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned());
    let mut cmd = Command::new(shell);
    cmd.arg("-lc").arg(script);
    cmd
}

/// Logs each line and returns the last one that wasn't blank.
async fn follow(
    stream: impl AsyncRead + Unpin,
    log: &Mutex<Option<Writer>>,
    label: Option<&str>,
) -> String {
    let mut lines = BufReader::new(stream).lines();
    let mut last = String::new();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        let step = Step {
            at: Utc::now(),
            kind: StepKind::Output,
            tool: label.map(str::to_owned),
            text: line.chars().take(LINE_CHARS).collect(),
        };
        // The log is a courtesy for the Agent tab; a line it missed is only missing there.
        if let Ok(mut guard) = log.lock()
            && let Some(w) = guard.as_mut()
        {
            drop(w.push(step));
        }
        last = line;
    }
    last
}

fn spawn(shell: &Shell<'_>) -> AppResult<Child> {
    let mut cmd = command(shell.script);
    cmd.current_dir(&shell.cwd)
        .envs(shell.env.iter().map(|(k, v)| (k, v)))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    Ok(cmd.spawn()?)
}

/// The line for a script that finished: its last output when it succeeded, else why it failed.
fn ending(status: ExitStatus, last_out: String, last_err: String) -> Exit {
    if status.success() {
        let line = if last_out.is_empty() {
            "done".to_owned()
        } else {
            last_out
        };
        return Exit { ok: true, line };
    }
    let code = status
        .code()
        .map_or_else(|| "a signal".to_owned(), |c| format!("code {c}"));
    let said = if last_err.is_empty() {
        last_out
    } else {
        last_err
    };
    Exit {
        ok: false,
        line: format!("exited with {code}: {said}"),
    }
}

/// Runs the script to its end, or stops it at the limit.
pub async fn run(shell: &Shell<'_>, log: Option<Writer>) -> AppResult<Exit> {
    let mut child = spawn(shell)?;
    let (out, err) = (child.stdout.take(), child.stderr.take());
    let log = Mutex::new(log);
    let finished = tokio::time::timeout(shell.limit, async {
        let (last_out, last_err, status) = tokio::join!(
            async {
                match out {
                    Some(s) => follow(s, &log, None).await,
                    None => String::new(),
                }
            },
            async {
                match err {
                    Some(s) => follow(s, &log, Some("stderr")).await,
                    None => String::new(),
                }
            },
            child.wait(),
        );
        (last_out, last_err, status)
    })
    .await;
    let Ok((last_out, last_err, status)) = finished else {
        let minutes = shell.limit.as_secs() / 60;
        return Ok(Exit {
            ok: false,
            line: format!("stopped after {minutes} minutes"),
        });
    };
    Ok(ending(status?, last_out, last_err))
}

#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;

    fn shell(script: &str, limit: Duration) -> Shell<'_> {
        Shell {
            script,
            cwd: std::env::temp_dir(),
            env: vec![("QD_TEST".to_owned(), "hello".to_owned())],
            limit,
        }
    }

    #[tokio::test]
    async fn reports_the_last_line_and_exit_code() {
        let ok = run(
            &shell("echo one; echo \"$QD_TEST\"", Duration::from_secs(20)),
            None,
        )
        .await
        .unwrap();
        assert_eq!(
            ok,
            Exit {
                ok: true,
                line: "hello".to_owned()
            }
        );
        let failed = run(
            &shell("echo nope >&2; exit 3", Duration::from_secs(20)),
            None,
        )
        .await
        .unwrap();
        assert_eq!(failed.line, "exited with code 3: nope");
        assert!(!failed.ok);
    }

    #[tokio::test]
    async fn stops_at_the_limit() {
        let slow = run(&shell("sleep 5", Duration::from_millis(200)), None)
            .await
            .unwrap();
        assert!(!slow.ok);
        assert!(slow.line.starts_with("stopped after"));
    }
}
