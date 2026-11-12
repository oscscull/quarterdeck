//! Changes `quarterdeck.toml` in place, keeping the user's comments and layout. Every edit is checked
//! by loading the result; a change that would break the file is undone.

use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, value};

use super::Config;
use super::calendar::{Feed, Origin};
use super::repos::Checkout;
use super::settings::{self, Area};
use super::sync::Peer;
use super::turns::TurnChange;
use crate::error::{AppError, AppResult};

fn path(config: &Config) -> std::path::PathBuf {
    config.root().join("quarterdeck.toml")
}

/// Applies `change` to the settings file, keeping it only if the result still loads.
fn edit(config: &Config, change: impl FnOnce(&mut DocumentMut) -> AppResult<()>) -> AppResult<()> {
    let before = std::fs::read_to_string(path(config))?;
    let mut doc: DocumentMut = before
        .parse()
        .map_err(|e| AppError::BadInput(format!("quarterdeck.toml: {e}")))?;
    change(&mut doc)?;
    std::fs::write(path(config), doc.to_string())?;
    if let Err(e) = settings::load(config) {
        std::fs::write(path(config), before)?;
        return Err(AppError::BadInput(format!(
            "that change would break quarterdeck.toml: {e}"
        )));
    }
    Ok(())
}

fn table<'a>(parent: &'a mut Table, name: &str) -> AppResult<&'a mut Table> {
    parent
        .entry(name)
        .or_insert_with(|| Item::Table(Table::new()))
        .as_table_mut()
        .ok_or_else(|| AppError::BadInput(format!("[{name}] in quarterdeck.toml isn't a table")))
}

fn tables<'a>(parent: &'a mut Table, name: &str) -> AppResult<&'a mut ArrayOfTables> {
    parent
        .entry(name)
        .or_insert_with(|| Item::ArrayOfTables(ArrayOfTables::new()))
        .as_array_of_tables_mut()
        .ok_or_else(|| {
            AppError::BadInput(format!("{name} in quarterdeck.toml isn't a list of tables"))
        })
}

/// Replaces the table whose `key` equals `id`, or adds `t` at the end.
fn replace_or_push(list: &mut ArrayOfTables, key: &str, id: &str, t: Table) {
    let at = list
        .iter()
        .position(|row| row.get(key).and_then(Item::as_str) == Some(id));
    match at.and_then(|i| list.get_mut(i)) {
        Some(existing) => *existing = t,
        None => list.push(t),
    }
}

fn strings(list: &[String]) -> Item {
    value(
        list.iter()
            .map(String::as_str)
            .collect::<toml_edit::Array>(),
    )
}

fn area_table(area: &Area) -> Table {
    let mut t = Table::new();
    t.insert("id", value(&area.id));
    t.insert("name", value(&area.name));
    t.insert("color", value(&area.color));
    t.insert("node", value(&area.node));
    t
}

/// Adds an area, or replaces the one with the same id.
pub fn upsert_area(config: &Config, area: &Area) -> AppResult<()> {
    edit(config, |doc| {
        let list = tables(doc.as_table_mut(), "areas")?;
        replace_or_push(list, "id", &area.id, area_table(area));
        Ok(())
    })
}

/// Replaces the whole areas list, e.g. with the one a peer sent.
pub fn set_areas(config: &Config, areas: &[Area]) -> AppResult<()> {
    edit(config, |doc| {
        let list = tables(doc.as_table_mut(), "areas")?;
        list.clear();
        for area in areas {
            list.push(area_table(area));
        }
        Ok(())
    })
}

/// Moves `[[projects]]`, the older name, to `[[repos]]` before either is edited.
fn rename_legacy_repos(doc: &mut DocumentMut) {
    if let Some(old) = doc.as_table_mut().remove("projects") {
        doc.as_table_mut().insert("repos", old);
    }
}

/// Adds a repo checkout, or replaces the one for the same repo.
pub fn upsert_repo(config: &Config, project: &Checkout) -> AppResult<()> {
    edit(config, |doc| {
        rename_legacy_repos(doc);
        let list = tables(doc.as_table_mut(), "repos")?;
        let mut t = Table::new();
        t.insert("repo", value(&project.repo));
        t.insert("path", value(&project.path));
        if !project.clean_after_run.is_empty() {
            t.insert("clean_after_run", strings(&project.clean_after_run));
        }
        replace_or_push(list, "repo", &project.repo, t);
        Ok(())
    })
}

pub fn remove_repo(config: &Config, repo: &str) -> AppResult<()> {
    edit(config, |doc| {
        rename_legacy_repos(doc);
        tables(doc.as_table_mut(), "repos")?
            .retain(|p| p.get("repo").and_then(Item::as_str) != Some(repo));
        Ok(())
    })
}

/// Adds a calendar feed, or replaces the one with the same name.
pub fn upsert_feed(config: &Config, feed: &Feed) -> AppResult<()> {
    edit(config, |doc| {
        let list = tables(table(doc.as_table_mut(), "calendar")?, "feeds")?;
        let mut t = Table::new();
        t.insert("name", value(&feed.name));
        match &feed.origin {
            Origin::Link { url } => t.insert("url", value(url)),
            Origin::Mac { mac_calendar } => t.insert("mac_calendar", value(mac_calendar)),
        };
        t.insert("area", value(&feed.area));
        t.insert("ignore_titles", strings(&feed.ignore_titles));
        replace_or_push(list, "name", &feed.name, t);
        Ok(())
    })
}

pub fn remove_feed(config: &Config, name: &str) -> AppResult<()> {
    edit(config, |doc| {
        tables(table(doc.as_table_mut(), "calendar")?, "feeds")?
            .retain(|f| f.get("name").and_then(Item::as_str) != Some(name));
        Ok(())
    })
}

pub fn set_calendar_ignores(
    config: &Config,
    titles: &[String],
    all_day: Option<bool>,
) -> AppResult<()> {
    edit(config, |doc| {
        let calendar = table(doc.as_table_mut(), "calendar")?;
        calendar.insert("ignore_titles", strings(titles));
        if let Some(skip) = all_day {
            calendar.insert("ignore_all_day", value(skip));
        }
        Ok(())
    })
}

/// Sends an owner's or repo's PRs to an area.
pub fn map_github(config: &Config, owner_or_repo: &str, area: &str) -> AppResult<()> {
    edit(config, |doc| {
        let github = table(doc.as_table_mut(), "github")?;
        let areas = github
            .entry("areas")
            .or_insert_with(|| Item::Table(Table::new()));
        areas[owner_or_repo] = value(area);
        Ok(())
    })
}

pub fn set_turns(config: &Config, change: TurnChange) -> AppResult<()> {
    change.check()?;
    edit(config, |doc| {
        let t = table(doc.as_table_mut(), "turns")?;
        let numbers = [
            ("agent_hours", change.agent_hours),
            ("you_remind_hours", change.you_remind_hours),
            ("others_hours", change.others_hours),
            ("later_hours", change.later_hours),
            ("quiet_minutes", change.quiet_minutes),
        ];
        for (key, n) in numbers {
            if let Some(n) = n {
                t.insert(key, value(n));
            }
        }
        if let Some(n) = change.failure_retries {
            t.insert("failure_retries", value(i64::from(n)));
        }
        if let Some(turn) = change.others_then {
            t.insert("others_then", value(turn.as_str()));
        }
        Ok(())
    })
}

/// What `set_sync` changes. Fields left `None` keep their current value.
#[derive(Debug, Clone, Copy)]
pub struct SyncChange {
    pub enabled: bool,
    pub port: Option<u16>,
    pub lan: Option<bool>,
}

pub fn set_sync(config: &Config, change: SyncChange) -> AppResult<()> {
    edit(config, |doc| {
        let sync = table(doc.as_table_mut(), "sync")?;
        sync.insert("enabled", value(change.enabled));
        if let Some(port) = change.port {
            sync.insert("port", value(i64::from(port)));
        }
        if let Some(lan) = change.lan {
            sync.insert("lan", value(lan));
        }
        Ok(())
    })
}

pub fn set_sync_token(config: &Config, token: &str) -> AppResult<()> {
    edit(config, |doc| {
        table(doc.as_table_mut(), "sync")?.insert("token", value(token));
        Ok(())
    })
}

/// Makes a copied config safe for a dev build: it serves nobody, reaches no peers (whose records
/// are real) and runs no Claude or scripts.
pub fn make_dev_safe(config: &Config) -> AppResult<()> {
    edit(config, |doc| {
        let sync = table(doc.as_table_mut(), "sync")?;
        sync.insert("enabled", value(false));
        sync.remove("peers");
        let dev = table(doc.as_table_mut(), "dev")?;
        dev.insert("run_claude", value(false));
        dev.insert("run_scripts", value(false));
        Ok(())
    })
}

/// Adds a peer, or replaces the one with the same name.
pub fn upsert_peer(config: &Config, peer: &Peer) -> AppResult<()> {
    edit(config, |doc| {
        let list = tables(table(doc.as_table_mut(), "sync")?, "peers")?;
        let mut t = Table::new();
        t.insert("name", value(&peer.name));
        t.insert("url", value(&peer.url));
        t.insert("token", value(&peer.token));
        replace_or_push(list, "name", &peer.name, t);
        Ok(())
    })
}

pub fn remove_peer(config: &Config, name: &str) -> AppResult<()> {
    edit(config, |doc| {
        tables(table(doc.as_table_mut(), "sync")?, "peers")?
            .retain(|p| p.get("name").and_then(Item::as_str) != Some(name));
        Ok(())
    })
}

pub fn set_github_self_review(config: &Config, list: &[String]) -> AppResult<()> {
    edit(config, |doc| {
        table(doc.as_table_mut(), "github")?.insert("self_review", strings(list));
        Ok(())
    })
}

/// Sets the informational checks for an owner or repo; an empty list removes the entry.
pub fn set_informational_checks(
    config: &Config,
    owner_or_repo: &str,
    checks: &[String],
) -> AppResult<()> {
    edit(config, |doc| {
        let github = table(doc.as_table_mut(), "github")?;
        let map = github
            .entry("informational_checks")
            .or_insert_with(|| Item::Table(Table::new()))
            .as_table_mut()
            .ok_or_else(|| {
                AppError::BadInput("[github.informational_checks] isn't a table".to_owned())
            })?;
        if checks.is_empty() {
            map.remove(owner_or_repo);
        } else {
            map.insert(owner_or_repo, strings(checks));
        }
        Ok(())
    })
}

/// Sets the items Claude works on one at a time, in order.
pub fn set_queue(config: &Config, items: &[String]) -> AppResult<()> {
    edit(config, |doc| {
        table(doc.as_table_mut(), "queue")?.insert("items", strings(items));
        Ok(())
    })
}

pub fn set_github_ignores(config: &Config, ignore: &[String]) -> AppResult<()> {
    edit(config, |doc| {
        table(doc.as_table_mut(), "github")?.insert("ignore", strings(ignore));
        Ok(())
    })
}

/// Adds an owner or repo to the ones whose issues become items, or takes it off.
pub fn set_issue_watch(config: &Config, owner_or_repo: &str, watch: bool) -> AppResult<()> {
    edit(config, |doc| {
        let issues = table(table(doc.as_table_mut(), "github")?, "issues")?;
        let mut list: Vec<String> = issues
            .get("watch")
            .and_then(Item::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        list.retain(|w| w != owner_or_repo);
        if watch {
            list.push(owner_or_repo.to_owned());
        }
        issues.insert("watch", strings(&list));
        Ok(())
    })
}

/// Sets one setting by its dotted path, e.g. `["github", "poll_minutes"]`.
pub fn set_value(config: &Config, path: &[&str], new: Item) -> AppResult<()> {
    let Some((last, parents)) = path.split_last() else {
        return Err(AppError::BadInput("empty setting path".to_owned()));
    };
    edit(config, |doc| {
        let mut at = doc.as_table_mut();
        for name in parents {
            at = table(at, name)?;
        }
        at.insert(last, new);
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_keep_comments_and_refuse_breakage() {
        let dir = std::env::temp_dir().join(format!("quarterdeck-edit-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        let config = Config::open(dir).unwrap();
        map_github(&config, "acme", "work").unwrap();
        let feed = Feed {
            name: "home".to_owned(),
            origin: Origin::Link {
                url: "https://x/ics".to_owned(),
            },
            area: "life".to_owned(),
            ignore_titles: vec![],
        };
        upsert_feed(&config, &feed).unwrap();
        upsert_feed(&config, &feed).unwrap();
        let text = std::fs::read_to_string(path(&config)).unwrap();
        assert!(text.contains("# Mechanical settings only"));
        let loaded = settings::load(&config).unwrap();
        assert_eq!(loaded.calendar.feeds.len(), 1);
        assert_eq!(loaded.github.area_for("acme/app"), "work");
        assert!(
            edit(&config, |doc| {
                doc["priority_limit"] = value("lots");
                Ok(())
            })
            .is_err()
        );
        assert_eq!(settings::load(&config).unwrap().priority_limit, 3);
    }

    #[test]
    fn issue_watches_and_plain_settings_change_by_name() {
        let dir = std::env::temp_dir().join(format!("quarterdeck-keys-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        let config = Config::open(dir).unwrap();
        set_issue_watch(&config, "me", true).unwrap();
        set_issue_watch(&config, "acme/app", true).unwrap();
        set_issue_watch(&config, "acme/app", false).unwrap();
        set_value(
            &config,
            &["github", "issues", "skip_labels"],
            strings(&["wontfix".to_owned()]),
        )
        .unwrap();
        set_value(&config, &["run_ttl_days"], value(3)).unwrap();
        let loaded = settings::load(&config).unwrap();
        assert_eq!(loaded.github.issues.watch, ["me"]);
        assert_eq!(loaded.github.issues.skip_labels, ["wontfix"]);
        assert_eq!(loaded.run_ttl_days, 3);
        assert!(set_value(&config, &["alerts", "enabled"], value("sometimes")).is_err());
    }
}
