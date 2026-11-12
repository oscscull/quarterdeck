//! Removes worktrees whose item is finished or gone.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::{folder, root};
use crate::cli;
use crate::domain::item::{Stage, WorkItem};
use crate::error::AppResult;

/// The repository a worktree belongs to, from its `.git` file: `gitdir: <repo>/.git/worktrees/<name>`.
fn repo_git_dir(worktree: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(worktree.join(".git")).ok()?;
    let gitdir = PathBuf::from(text.strip_prefix("gitdir:")?.trim());
    Some(gitdir.parent()?.parent()?.to_path_buf())
}

fn remove(worktree: &Path) -> AppResult<()> {
    if let Some(git_dir) = repo_git_dir(worktree) {
        let removed = Command::new(cli::resolve("git"))
            .arg("--git-dir")
            .arg(&git_dir)
            .args(["worktree", "remove", "--force"])
            .arg(worktree)
            .status()?;
        if removed.success() {
            return Ok(());
        }
    }
    // Not a worktree git knows about any more: just a folder.
    Ok(std::fs::remove_dir_all(worktree)?)
}

pub fn sweep(items: &[WorkItem]) -> AppResult<()> {
    let root = root()?;
    let Ok(entries) = std::fs::read_dir(&root) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let open = items
            .iter()
            .any(|i| folder(&i.id) == name && i.stage() != Stage::Done);
        if !open {
            remove(&entry.path())?;
        }
    }
    Ok(())
}
