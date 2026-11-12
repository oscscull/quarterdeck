//! Deletes a project's `clean_after_run` folders from a worktree once a run there ends, so worktrees
//! waiting on review don't hold build output.

use std::path::{Component, Path, PathBuf};

use super::Workspace;
use crate::error::{AppError, AppResult};

/// `rel` inside `root`, refusing anything that could reach outside it.
fn inside(root: &Path, rel: &str) -> AppResult<PathBuf> {
    let path = Path::new(rel);
    let plain = path.components().all(|c| matches!(c, Component::Normal(_)));
    if rel.trim().is_empty() || !plain {
        return Err(AppError::BadInput(format!(
            "clean_after_run entry '{rel}' must be a plain path inside the repo"
        )));
    }
    Ok(root.join(path))
}

fn remove(root: &Path, rel: &str) -> AppResult<()> {
    let path = inside(root, rel)?;
    // A symlink is unlinked, never followed.
    match std::fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
        Ok(m) if m.is_dir() => Ok(std::fs::remove_dir_all(&path)?),
        Ok(_) => Ok(std::fs::remove_file(&path)?),
    }
}

/// Deletes every listed folder, carrying on past one that fails and reporting the first failure.
pub fn clean(ws: &Workspace) -> AppResult<()> {
    let mut first = Ok(());
    for rel in &ws.clean_after_run {
        if let Err(e) = remove(&ws.dir, rel)
            && first.is_ok()
        {
            first = Err(e);
        }
    }
    first
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_paths_inside_the_worktree() {
        let root = Path::new("/wt");
        assert_eq!(
            inside(root, "rustapp/target").unwrap(),
            PathBuf::from("/wt/rustapp/target")
        );
        for bad in ["", "/etc", "../x", "a/../../x", "./target"] {
            assert!(inside(root, bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn removes_listed_folders_and_keeps_the_rest() {
        let dir = std::env::temp_dir().join(format!("qd-clean-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("app/target/debug")).unwrap();
        std::fs::write(dir.join("app/target/debug/big"), "x").unwrap();
        std::fs::write(dir.join("app/main.rs"), "x").unwrap();
        let ws = Workspace {
            project: PathBuf::new(),
            repo: String::new(),
            kind: super::super::Kind::Pr(1),
            dir: dir.clone(),
            branch: None,
            clean_after_run: vec!["app/target".to_owned(), "missing".to_owned()],
        };
        clean(&ws).unwrap();
        assert!(!dir.join("app/target").exists());
        assert!(dir.join("app/main.rs").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
