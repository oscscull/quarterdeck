//! Default config files, embedded in the binary and written on first run. A file Quarterdeck wrote and
//! nobody has edited since follows newer defaults; once edited, it is never overwritten.

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::AppResult;

macro_rules! default_file {
    ($path:literal) => {
        (
            $path,
            include_str!(concat!("../../defaults/config/", $path)),
        )
    };
}

const FILES: [(&str, &str); 18] = [
    default_file!("README.md"),
    default_file!("quarterdeck.toml"),
    default_file!("prompts/chat.md"),
    default_file!("prompts/draft-schedule.md"),
    default_file!("prompts/resolve-urgent.md"),
    default_file!("prompts/run-schedule.md"),
    default_file!("prompts/continue.md"),
    default_file!("prompts/hand-over.md"),
    default_file!("prompts/fix-ci.md"),
    default_file!("prompts/fix-conflicts.md"),
    default_file!("prompts/ci-done.md"),
    default_file!("prompts/answer.md"),
    default_file!("prompts/message.md"),
    default_file!("prompts/workspace.md"),
    default_file!("prompts/issue-workspace.md"),
    default_file!("prompts/new-workspace.md"),
    default_file!("prompts/no-workspace.md"),
    default_file!("prompts/queue.md"),
];

/// Folders that must exist even when empty.
const DIRS: [&str; 2] = ["schedules", "inbox"];

/// Quarterdeck's record of what it last wrote to each default file, as content fingerprints.
const LEDGER: &str = ".quarterdeck-defaults";

/// Rewritten freely by people and agents, so never replaced once it exists.
const NEVER_REPLACED: &str = "quarterdeck.toml";

/// FNV-1a: stable across builds, unlike the standard library's hasher.
fn fingerprint(text: &str) -> String {
    let hash = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{hash:016x}")
}

fn read_ledger(root: &Path) -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(root.join(LEDGER)).unwrap_or_default();
    text.lines()
        .filter_map(|l| l.split_once(' '))
        .map(|(path, hash)| (path.to_owned(), hash.to_owned()))
        .collect()
}

fn write_ledger(root: &Path, ledger: &BTreeMap<String, String>) -> AppResult<()> {
    let header = "# Written by Quarterdeck: which default files it wrote, so it can update the unedited ones.\n";
    let text = ledger.iter().fold(header.to_owned(), |acc, (path, hash)| {
        acc + path + " " + hash + "\n"
    });
    Ok(std::fs::write(root.join(LEDGER), text)?)
}

/// Whether to (re)write a default: missing, or unedited since Quarterdeck wrote an older version.
fn should_write(current: Option<&str>, recorded: Option<&String>, relative: &str) -> bool {
    match current {
        None => true,
        Some(_) if relative == NEVER_REPLACED => false,
        Some(text) => recorded == Some(&fingerprint(text)),
    }
}

pub fn write_missing(root: &Path) -> AppResult<()> {
    for dir in DIRS {
        std::fs::create_dir_all(root.join(dir))?;
    }
    let mut ledger = read_ledger(root);
    for (relative, contents) in FILES {
        let path = root.join(relative);
        let current = std::fs::read_to_string(&path).ok();
        if current.as_deref() == Some(contents) {
            ledger.insert(relative.to_owned(), fingerprint(contents));
            continue;
        }
        if !should_write(current.as_deref(), ledger.get(relative), relative) {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, contents)?;
        ledger.insert(relative.to_owned(), fingerprint(contents));
    }
    write_ledger(root, &ledger)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_unedited_defaults_follow_updates() {
        let old = fingerprint("old default");
        assert!(should_write(None, None, "prompts/a.md"));
        assert!(should_write(
            Some("old default"),
            Some(&old),
            "prompts/a.md"
        ));
        assert!(!should_write(Some("my edit"), Some(&old), "prompts/a.md"));
        assert!(!should_write(Some("unknown"), None, "prompts/a.md"));
        assert!(!should_write(
            Some("old default"),
            Some(&old),
            "quarterdeck.toml"
        ));
    }
}
