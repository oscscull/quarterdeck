//! An issue's text, fetched by Quarterdeck for the run that fixes it: a project's sandbox may keep
//! Claude off GitHub.

use serde::Deserialize;

use super::Workspace;
use super::git::run;
use crate::error::AppResult;

#[derive(Debug, Deserialize)]
struct Author {
    login: String,
}

#[derive(Debug, Deserialize)]
struct Comment {
    author: Author,
    body: String,
}

#[derive(Debug, Deserialize)]
struct Issue {
    title: String,
    body: String,
    comments: Vec<Comment>,
}

/// The issue's title, description and comments, as plain text.
pub async fn text(ws: &Workspace, number: u64) -> AppResult<String> {
    let shown = number.to_string();
    let view = [
        "issue",
        "view",
        shown.as_str(),
        "--repo",
        ws.repo.as_str(),
        "--json",
        "title,body,comments",
    ];
    let json = run("gh", &ws.project, &view).await?;
    Ok(render(number, &serde_json::from_str(&json)?))
}

fn render(number: u64, issue: &Issue) -> String {
    let head = format!("Issue #{number}: {}\n\n{}", issue.title, issue.body.trim());
    issue.comments.iter().fold(head, |text, c| {
        text + "\n\n" + &c.author.login + " commented:\n" + c.body.trim()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_the_issue_and_its_comments() {
        let issue: Issue = serde_json::from_str(
            r#"{"title":"Upsert","body":"Breaks.\n","comments":[{"author":{"login":"o"},"body":"Still."}]}"#,
        )
        .unwrap();
        assert_eq!(
            render(3, &issue),
            "Issue #3: Upsert\n\nBreaks.\n\no commented:\nStill."
        );
    }
}
