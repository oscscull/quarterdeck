//! Reads the open issues of watched owners and repos with GraphQL searches through `gh`.

use serde::Deserialize;

use super::super::fetch::{Nodes, Repo, gh};
use crate::error::AppResult;

const QUERY: &str = "query($q: String!, $after: String) { search(query: $q, type: ISSUE, first: 100, \
    after: $after) { pageInfo { hasNextPage endCursor } nodes { ... on Issue { number title url \
    repository { nameWithOwner } labels(first: 20) { nodes { name } } } } } }";

/// Enough pages for any repo a person triages by hand; past it, the rest wait for issues to close.
const MAX_PAGES: usize = 10;

/// One open issue, flattened to what Quarterdeck needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub url: String,
    pub labels: Vec<String>,
}

impl Issue {
    /// `owner/repo#12`.
    pub fn external_id(&self) -> String {
        format!("{}#{}", self.repo, self.number)
    }
}

/// The search for one `watch` entry: a repo (`me/app`) or an owner (`me`).
fn query(entry: &str) -> String {
    let scope = if entry.contains('/') { "repo" } else { "user" };
    format!("q=is:issue is:open archived:false {scope}:{entry}")
}

/// Open issues of every watched owner and repo, each once.
pub async fn open_issues(watch: &[String]) -> AppResult<Vec<Issue>> {
    let mut all: Vec<Issue> = Vec::new();
    for entry in watch {
        for issue in search(&query(entry)).await? {
            if !all.iter().any(|i| i.external_id() == issue.external_id()) {
                all.push(issue);
            }
        }
    }
    Ok(all)
}

async fn search(q: &str) -> AppResult<Vec<Issue>> {
    let mut found = Vec::new();
    let mut after = "after=null".to_owned();
    for _ in 0..MAX_PAGES {
        let args = [
            "api",
            "graphql",
            "-f",
            &format!("query={QUERY}"),
            "-f",
            q,
            "-F",
            &after,
        ];
        let page: Response = serde_json::from_str(&gh(&args).await?)?;
        let search = page.data.search;
        found.extend(search.nodes.into_iter().filter_map(Node::into_issue));
        match search.page_info.end_cursor {
            Some(cursor) if search.page_info.has_next_page => after = format!("after={cursor}"),
            _ => break,
        }
    }
    Ok(found)
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
#[serde(rename_all = "camelCase")]
struct Search {
    page_info: PageInfo,
    nodes: Vec<Node>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct PageInfo {
    has_next_page: bool,
    end_cursor: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Label {
    name: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Node {
    number: Option<u64>,
    title: String,
    url: String,
    repository: Option<Repo>,
    labels: Nodes<Label>,
}

impl Node {
    fn into_issue(self) -> Option<Issue> {
        Some(Issue {
            repo: self.repository?.name_with_owner,
            number: self.number?,
            title: self.title,
            url: self.url,
            labels: self.labels.nodes.into_iter().map(|l| l.name).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owners_and_repos_search_differently() {
        assert!(query("me").ends_with("user:me"));
        assert!(query("me/app").ends_with("repo:me/app"));
    }

    #[test]
    fn reads_a_page() {
        let json = r#"{"data":{"search":{"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[
            {"number":7,"title":"Breaks","url":"u","repository":{"nameWithOwner":"me/app"},
             "labels":{"nodes":[{"name":"bug"}]}},{}]}}}"#;
        let page: Response = serde_json::from_str(json).unwrap();
        let issues: Vec<Issue> = page
            .data
            .search
            .nodes
            .into_iter()
            .filter_map(Node::into_issue)
            .collect();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].external_id(), "me/app#7");
        assert_eq!(issues[0].labels, ["bug"]);
    }
}
