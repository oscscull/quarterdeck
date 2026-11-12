//! Asking the user's login shell where a command is, with a deadline: a login profile can hang on
//! a keychain prompt or a slow version manager. The shell runs interactive too, as version
//! managers like nvm are often set up in `.zshrc`, which a login shell alone doesn't read.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const SHELL_LIMIT: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub enum Lookup {
    Found(PathBuf),
    Missing,
    /// It didn't answer in time, or couldn't start, which says nothing about the tool.
    NoAnswer,
}

pub fn lookup(command: &str) -> Lookup {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_owned());
    let Ok(mut child) = Command::new(shell)
        .args(["-ilc", &format!("command -v '{command}'")])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return Lookup::NoAnswer;
    };
    let started = Instant::now();
    while matches!(child.try_wait(), Ok(None)) {
        if started.elapsed() > SHELL_LIMIT {
            drop(child.kill());
            return Lookup::NoAnswer;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let Ok(out) = child.wait_with_output() else {
        return Lookup::NoAnswer;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let path = PathBuf::from(text.lines().last().unwrap_or_default().trim());
    if path.is_absolute() && path.exists() {
        Lookup::Found(path)
    } else {
        Lookup::Missing
    }
}
