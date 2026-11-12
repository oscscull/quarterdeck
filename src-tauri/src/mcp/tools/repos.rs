use schemars::JsonSchema;
use serde::Deserialize;

use super::super::ctx::Ctx;
use super::super::tool::{NoArgs, Tool, tool};
use crate::config::edit;
use crate::config::repos::Checkout;
use crate::error::{AppError, AppResult};

const SET: &str = "Tells Quarterdeck where a GitHub repo is checked out on this machine, e.g. repo \
\"me/webapp\", path \"~/code/webapp\". Work Quarterdeck starts on that repo's PRs (fixing CI, acting \
on the user's answers), and on items linking one of its issues, then runs in that checkout, in a git \
worktree of its own per item, with the repo's own Claude settings and permissions. \
`clean_after_run` lists folders, relative to the repo root, deleted from the worktree after every \
run (build output such as `target` or `dist`) so idle worktrees don't hold gigabytes. Replaces the \
repo's entry if it is already set, so pass every field you want to keep.";

pub fn tools() -> Vec<Tool> {
    vec![
        tool(
            "list_repos",
            "Repos Quarterdeck knows a checkout of, and where.",
            list,
        ),
        tool("set_repo", SET, set),
        tool(
            "remove_repo",
            "Forgets a repo's checkout, by `owner/repo`.",
            remove,
        ),
    ]
}

fn list(ctx: &Ctx, _: NoArgs) -> AppResult<Vec<Checkout>> {
    Ok(ctx.settings()?.repos)
}

fn set(ctx: &Ctx, project: Checkout) -> AppResult<()> {
    if !project.dir().join(".git").exists() {
        return Err(AppError::BadInput(format!(
            "{} isn't a git checkout",
            project.dir().display()
        )));
    }
    edit::upsert_repo(&ctx.config, &project)
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RepoArgs {
    repo: String,
}

fn remove(ctx: &Ctx, args: RepoArgs) -> AppResult<()> {
    edit::remove_repo(&ctx.config, &args.repo)
}
