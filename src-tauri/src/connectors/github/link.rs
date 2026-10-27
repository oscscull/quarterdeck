//! Reads which repo, issue or PR a GitHub link points at.

/// `owner/repo` and the number from `https://github.com/<owner>/<repo>/<section>/<n>`, where
/// `section` is `issues` or `pull`, with any trailing `/`, `#…` or `?…` ignored.
pub fn parse(link: &str, section: &str) -> Option<(String, u64)> {
    let path = link.trim().strip_prefix("https://github.com/")?;
    let mut parts = path.split('/');
    let (owner, repo) = (parts.next()?, parts.next()?);
    if parts.next()? != section || owner.is_empty() || repo.is_empty() {
        return None;
    }
    let digits: String = parts
        .next()?
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    Some((format!("{owner}/{repo}"), digits.parse().ok()?))
}

/// `owner/repo` from a link to the repo itself, `https://github.com/<owner>/<repo>`, with any
/// trailing `/`, `.git`, `#…` or `?…` ignored. Links to a page inside the repo aren't the repo.
pub fn repo(link: &str) -> Option<String> {
    let path = link.trim().strip_prefix("https://github.com/")?;
    let path = path.split(['?', '#']).next()?.trim_end_matches('/');
    let (owner, repo) = path.split_once('/')?;
    let repo = repo.strip_suffix(".git").unwrap_or(repo);
    let named = |s: &str| !s.is_empty() && !s.contains('/');
    (named(owner) && named(repo)).then(|| format!("{owner}/{repo}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_links_to_the_repo_itself() {
        assert_eq!(
            repo("https://github.com/me/app/").as_deref(),
            Some("me/app")
        );
        assert_eq!(repo("https://github.com/me/app/pull/2"), None);
        assert_eq!(repo("https://github.com/me"), None);
        assert_eq!(repo("https://github.com/me/"), None);
    }

    #[test]
    fn reads_the_repo_and_number_of_its_section() {
        let pr = Some(("me/app".to_owned(), 24));
        for link in [
            "https://github.com/me/app/pull/24",
            "https://github.com/me/app/pull/24/files",
            " https://github.com/me/app/pull/24#issuecomment-1 ",
        ] {
            assert_eq!(parse(link, "pull"), pr, "{link}");
        }
        assert_eq!(parse("https://github.com/me/app/issues/24", "pull"), None);
        assert_eq!(parse("https://github.com/me/app/pull/", "pull"), None);
        assert_eq!(parse("https://example.com/me/app/pull/24", "pull"), None);
    }
}
