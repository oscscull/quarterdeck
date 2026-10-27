//! Creating worktrees with `git`, and finding a PR's branch or a repo's default branch with `gh`.

use std::path::Path;

use tokio::process::Command;

use super::{Kind, Workspace};
use crate::cli;
use crate::error::{AppError, AppResult};

pub(super) async fn run(program: &str, dir: &Path, args: &[&str]) -> AppResult<String> {
    let out = Command::new(cli::resolve(program))
        .current_dir(dir)
        .args(args)
        .output()
        .await?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_owned();
        Err(AppError::Connector(format!(
            "{program} {}: {err}",
            args.join(" ")
        )))
    }
}

/// The PR's head branch.
pub(super) async fn pr_branch(ws: &Workspace, number: u64) -> AppResult<String> {
    let number = number.to_string();
    let repo = ws.repo.as_str();
    let view = [
        "pr",
        "view",
        number.as_str(),
        "--repo",
        repo,
        "--json",
        "headRefName",
        "--jq",
        ".headRefName",
    ];
    run("gh", &ws.project, &view).await
}

pub(super) async fn default_branch(ws: &Workspace) -> AppResult<String> {
    let jq = ".defaultBranchRef.name";
    let view = [
        "repo",
        "view",
        ws.repo.as_str(),
        "--json",
        "defaultBranchRef",
        "--jq",
        jq,
    ];
    run("gh", &ws.project, &view).await
}

/// Makes the item's worktree, unless it exists.
pub async fn prepare(ws: &Workspace) -> AppResult<()> {
    if !ws.dir.exists() {
        match (ws.kind, ws.branch.as_deref()) {
            (Kind::Pr(number), _) => prepare_pr(ws, number).await?,
            (_, Some(branch)) => prepare_branch(ws, branch).await?,
            (_, None) => return Err(AppError::Connector("no branch to work on".to_owned())),
        }
    }
    copy_local_settings(ws);
    Ok(())
}

/// At the PR's latest commit. The head is detached so the branch can stay checked out in the
/// user's own checkout; pushes go to `HEAD:<branch>`.
async fn prepare_pr(ws: &Workspace, number: u64) -> AppResult<()> {
    let branch = pr_branch(ws, number).await?;
    run("git", &ws.project, &["fetch", "origin", branch.as_str()]).await?;
    let dir = ws.dir.display().to_string();
    let start = format!("origin/{branch}");
    let add = ["worktree", "add", "--detach", dir.as_str(), start.as_str()];
    run("git", &ws.project, &add).await.map(drop)
}

/// On the work's own branch: reused if an earlier worktree left it, otherwise new from the latest
/// default branch, with no upstream so nothing pushes to the default branch by accident.
async fn prepare_branch(ws: &Workspace, branch: &str) -> AppResult<()> {
    let base = default_branch(ws).await?;
    run("git", &ws.project, &["fetch", "origin", base.as_str()]).await?;
    let dir = ws.dir.display().to_string();
    let local = format!("refs/heads/{branch}");
    let verify = ["rev-parse", "--verify", "--quiet", local.as_str()];
    let exists = run("git", &ws.project, &verify).await.is_ok();
    let start = format!("origin/{base}");
    let (dir, start) = (dir.as_str(), start.as_str());
    let add = if exists {
        vec!["worktree", "add", dir, branch]
    } else {
        vec!["worktree", "add", "--no-track", "-b", branch, dir, start]
    };
    run("git", &ws.project, &add).await.map(drop)
}

/// The user's personal Claude permissions for the project aren't in git, so bring them along on
/// every run. Contents only: `fs::copy` also copies metadata, which macOS can refuse. Without
/// them the run just asks for less, so it goes on.
fn copy_local_settings(ws: &Workspace) {
    let local = Path::new(".claude").join("settings.local.json");
    let Ok(text) = std::fs::read(ws.project.join(&local)) else {
        return;
    };
    let to = ws.dir.join(&local);
    if let Some(parent) = to.parent() {
        drop(std::fs::create_dir_all(parent));
    }
    drop(std::fs::write(to, text));
}
