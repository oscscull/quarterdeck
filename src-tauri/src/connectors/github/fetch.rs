//! Reads the user's open PRs with one GraphQL query through the `gh` command.

use serde::Deserialize;
use tokio::process::Command;

use super::checks::{self, Check};
use crate::cli;
use crate::error::{AppError, AppResult};

const QUERY: &str = "query($q: String!) { search(query: $q, type: ISSUE, first: 50) { nodes { \
    ... on PullRequest { number title url isDraft repository { nameWithOwner } reviewDecision \
    mergeable mergeStateStatus \
    author { login } \
    closingIssuesReferences(first: 20) { nodes { number repository { nameWithOwner } } } \
    reviewRequests(first: 20) { nodes { requestedReviewer { \
      ... on User { login } ... on Team { name } ... on Bot { login } } } } \
    latestReviews(first: 20) { nodes { author { __typename login } } } \
    comments { totalCount } reviews { totalCount } \
    reviewThreads(first: 100) { nodes { comments { totalCount } } } \
    commits(last: 1) { nodes { commit { statusCheckRollup { state contexts(first: 100) { nodes { \
      __typename ... on CheckRun { name status conclusion } \
      ... on StatusContext { context state description } } } } } } } } } } }";

/// One open PR, flattened to what Quarterdeck needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pr {
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub url: String,
    pub draft: bool,
    /// `APPROVED`, `CHANGES_REQUESTED`, `REVIEW_REQUIRED`, or none.
    pub review: Option<String>,
    /// `SUCCESS`, `FAILURE`, `ERROR`, `PENDING`, or none.
    pub ci: Option<String>,
    /// Each check on the head commit, for leaving out informational ones.
    pub checks: Vec<Check>,
    /// Results of informational checks, noted on the item rather than counted as CI.
    pub notes: Vec<String>,
    /// Comments, reviews and review-thread replies together.
    pub comments: u64,
    pub reviewers: Reviewers,
    /// People asked to review it, or who already have (GitHub drops a request once it's reviewed).
    /// Bots and the author aren't counted.
    pub reviewer_names: Vec<String>,
    /// Issues it closes when merged, as `owner/repo#12`.
    pub closes: Vec<String>,
    pub base: Base,
}

/// How the PR's branch stands against the branch it merges into.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Base {
    /// Up to date, or GitHub doesn't require it to be.
    #[default]
    Current,
    /// Behind, and the repo only merges branches that are up to date.
    Behind,
    /// It can't merge cleanly.
    Conflicts,
}

impl Base {
    /// From GitHub's `mergeable` and `mergeStateStatus`. GitHub reports `BEHIND` only where branch
    /// protection requires branches to be up to date.
    fn of(mergeable: Option<&str>, state: Option<&str>) -> Self {
        match (mergeable, state) {
            (Some("CONFLICTING"), _) | (_, Some("DIRTY")) => Self::Conflicts,
            (_, Some("BEHIND")) => Self::Behind,
            _ => Self::Current,
        }
    }
}

/// Who is expected to review a PR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reviewers {
    /// Someone was asked to review it, or already has.
    Requested,
    /// Nobody has been asked or reviewed yet.
    NoneRequested,
    /// The user reviews their own PRs in this repo (from config, not GitHub).
    Yourself,
}

pub(super) async fn gh(args: &[&str]) -> AppResult<String> {
    let out = Command::new(cli::resolve("gh")).args(args).output().await?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
    } else {
        Err(AppError::Connector(format!(
            "gh: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )))
    }
}

pub async fn open_prs() -> AppResult<Vec<Pr>> {
    let login = gh(&["api", "user", "--jq", ".login"]).await?;
    let q = format!("q=is:pr is:open archived:false author:{login}");
    let json = gh(&["api", "graphql", "-f", &format!("query={QUERY}"), "-f", &q]).await?;
    let parsed: Response = serde_json::from_str(&json)?;
    Ok(parsed
        .data
        .search
        .nodes
        .into_iter()
        .filter_map(Node::into_pr)
        .collect())
}

#[derive(Debug, Deserialize)]
struct Response {
    data: Data,
}

#[derive(Debug, Deserialize)]
struct Data {
    search: Search,
}

#[derive(Debug, Deserialize)]
struct Search {
    nodes: Vec<Node>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Node {
    number: Option<u64>,
    title: String,
    url: String,
    is_draft: bool,
    repository: Option<Repo>,
    review_decision: Option<String>,
    mergeable: Option<String>,
    merge_state_status: Option<String>,
    author: Option<Login>,
    closing_issues_references: Nodes<IssueRef>,
    review_requests: Nodes<Request>,
    latest_reviews: Nodes<Review>,
    comments: Count,
    reviews: Count,
    review_threads: Nodes<Thread>,
    commits: Nodes<CommitNode>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct IssueRef {
    number: u64,
    repository: Repo,
}

impl IssueRef {
    fn external_id(&self) -> String {
        format!("{}#{}", self.repository.name_with_owner, self.number)
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(super) struct Repo {
    pub(super) name_with_owner: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Count {
    total_count: u64,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub(super) struct Nodes<T> {
    pub(super) nodes: Vec<T>,
}

impl<T> Default for Nodes<T> {
    fn default() -> Self {
        Self { nodes: Vec::new() }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct Login {
    pub(super) login: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Request {
    requested_reviewer: Option<Reviewer>,
}

/// A user or bot (`login`) or a team (`name`).
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Reviewer {
    login: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Review {
    author: Option<Author>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Author {
    #[serde(rename = "__typename")]
    kind: String,
    login: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Thread {
    comments: Count,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct CommitNode {
    commit: Commit,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Commit {
    status_check_rollup: Option<Rollup>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Rollup {
    state: String,
    contexts: Nodes<CheckNode>,
}

/// A check run (`name`, `status`, `conclusion`) or a commit status (`context`, `state`).
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct CheckNode {
    #[serde(rename = "__typename")]
    kind: String,
    name: String,
    status: String,
    conclusion: Option<String>,
    context: String,
    state: String,
    description: Option<String>,
}

impl CheckNode {
    fn into_check(self) -> Check {
        if self.kind == "StatusContext" {
            Check {
                outcome: checks::status_outcome(&self.state),
                name: self.context,
                detail: self.description.unwrap_or_default(),
            }
        } else {
            Check {
                outcome: checks::run_outcome(&self.status, self.conclusion.as_deref()),
                name: self.name,
                detail: String::new(),
            }
        }
    }
}

/// The head commit's rollup state and its individual checks.
fn ci_of(commits: Nodes<CommitNode>) -> (Option<String>, Vec<Check>) {
    let rollup = commits
        .nodes
        .into_iter()
        .next()
        .and_then(|c| c.commit.status_check_rollup);
    rollup.map_or((None, Vec::new()), |r| {
        let checks = r
            .contexts
            .nodes
            .into_iter()
            .map(CheckNode::into_check)
            .collect();
        (Some(r.state), checks)
    })
}

impl Node {
    /// Requested reviewers, then humans who reviewed, without the author or duplicates.
    fn reviewer_names(&self) -> Vec<String> {
        let author = self.author.as_ref().map(|a| a.login.as_str());
        let requested = self
            .review_requests
            .nodes
            .iter()
            .filter_map(|r| r.requested_reviewer.as_ref())
            .filter_map(|r| r.login.clone().or_else(|| r.name.clone()));
        let reviewed = self
            .latest_reviews
            .nodes
            .iter()
            .filter_map(|r| r.author.as_ref())
            .filter(|a| a.kind != "Bot")
            .map(|a| a.login.clone());
        let mut names: Vec<String> = Vec::new();
        for name in requested.chain(reviewed) {
            if Some(name.as_str()) != author && !names.contains(&name) {
                names.push(name);
            }
        }
        names
    }

    fn into_pr(self) -> Option<Pr> {
        let reviewer_names = self.reviewer_names();
        let threads: u64 = self
            .review_threads
            .nodes
            .iter()
            .map(|t| t.comments.total_count)
            .sum();
        let (ci, checks) = ci_of(self.commits);
        let base = Base::of(
            self.mergeable.as_deref(),
            self.merge_state_status.as_deref(),
        );
        let closes = self
            .closing_issues_references
            .nodes
            .iter()
            .map(IssueRef::external_id)
            .collect();
        Some(Pr {
            repo: self.repository?.name_with_owner,
            number: self.number?,
            title: self.title,
            url: self.url,
            draft: self.is_draft,
            review: self.review_decision,
            ci,
            checks,
            notes: Vec::new(),
            comments: self.comments.total_count + self.reviews.total_count + threads,
            reviewers: if reviewer_names.is_empty() {
                Reviewers::NoneRequested
            } else {
                Reviewers::Requested
            },
            reviewer_names,
            closes,
            base,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_reads_github_merge_state() {
        assert_eq!(
            Base::of(Some("CONFLICTING"), Some("BLOCKED")),
            Base::Conflicts
        );
        assert_eq!(Base::of(Some("UNKNOWN"), Some("DIRTY")), Base::Conflicts);
        assert_eq!(Base::of(Some("MERGEABLE"), Some("BEHIND")), Base::Behind);
        assert_eq!(Base::of(Some("MERGEABLE"), Some("BLOCKED")), Base::Current);
        assert_eq!(Base::of(None, None), Base::Current);
    }
}
