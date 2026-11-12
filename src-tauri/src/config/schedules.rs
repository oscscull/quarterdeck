//! Schedules stored as `schedules/<id>.md`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use chrono::Local;

use super::{Config, frontmatter};
use crate::cron::explain::explain;
use crate::domain::fired;
use crate::domain::schedule::Schedule;
use crate::domain::script::Script;
use crate::domain::trigger::Trigger;
use crate::error::{AppError, AppResult};

/// The TOML part of a schedule file. Files from before schedules had several triggers have one
/// `trigger` instead; it's read into `triggers` and written back that way.
#[derive(Debug, Serialize, Deserialize)]
struct Settings {
    name: String,
    area: String,
    enabled: bool,
    #[serde(default)]
    request: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    allowed_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    script: Option<Script>,
    #[serde(default, skip_serializing)]
    trigger: Option<Trigger>,
    #[serde(default)]
    triggers: Vec<Trigger>,
}

impl Settings {
    fn all_triggers(&mut self) -> Vec<Trigger> {
        let old = self.trigger.take();
        std::mem::take(&mut self.triggers)
            .into_iter()
            .chain(old)
            .map(Trigger::normalised)
            .collect()
    }
}

/// Schedules that loaded, plus a message for each file that did not.
#[derive(Debug, Default)]
pub struct Loaded {
    pub schedules: Vec<Schedule>,
    pub problems: Vec<String>,
}

fn dir(config: &Config) -> PathBuf {
    config.root().join("schedules")
}

fn path(config: &Config, id: &str) -> PathBuf {
    dir(config).join(format!("{id}.md"))
}

/// Ids come from the UI, so only plain slugs may become file paths.
fn checked_path(config: &Config, id: &str) -> AppResult<PathBuf> {
    if !id.is_empty() && slug(id) == id {
        Ok(path(config, id))
    } else {
        Err(AppError::NotFound(format!("schedule {id}")))
    }
}

/// Reads every schedule file, sorted by name. A broken file never hides the others.
pub fn list(config: &Config) -> AppResult<Loaded> {
    let mut loaded = Loaded::default();
    for entry in std::fs::read_dir(dir(config))? {
        let file = entry?.path();
        if file.extension().is_some_and(|e| e == "md") {
            match read(&file) {
                Ok(s) => loaded.schedules.push(s),
                Err(e) => loaded.problems.push(format!("{}: {e}", file.display())),
            }
        }
    }
    loaded.schedules.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(loaded)
}

pub fn find(config: &Config, id: &str) -> AppResult<Schedule> {
    read(&checked_path(config, id)?)
}

pub fn delete(config: &Config, id: &str) -> AppResult<()> {
    std::fs::remove_file(checked_path(config, id)?)?;
    Ok(())
}

fn read(file: &Path) -> AppResult<Schedule> {
    let id = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let text = std::fs::read_to_string(file)?;
    let (front, body) = frontmatter::split(&text)
        .ok_or_else(|| AppError::NotFound(format!("settings block (+++) in {id}.md")))?;
    let mut s: Settings = toml::from_str(front)?;
    Ok(Schedule {
        id,
        triggers: s.all_triggers(),
        script: s.script.take(),
        name: s.name,
        prompt: body.to_owned(),
        area: s.area,
        enabled: s.enabled,
        request: s.request,
        allowed_tools: s.allowed_tools,
    })
}

fn watchable(path: Option<&String>) -> bool {
    path.is_some_and(|p| !p.is_empty() && !p.contains('*'))
}

/// Refuses triggers that could never fire: a bad cron expression or an unknown event.
pub fn check(schedule: &Schedule) -> AppResult<()> {
    for trigger in &schedule.triggers {
        match trigger {
            Trigger::Cron { expr } => drop(explain(expr, Local::now(), 1)?),
            Trigger::Event(e) if !fired::known(&e.on) => {
                return Err(AppError::BadInput(format!(
                    "no event is called {}: see list_event_kinds, or start custom ones with {}",
                    e.on,
                    fired::CUSTOM
                )));
            }
            Trigger::Event(e) if e.on == fired::FILE_CHANGED && !watchable(e.when.get("path")) => {
                return Err(AppError::BadInput(format!(
                    "a {} trigger needs `when.path`: the file or folder to watch, with no `*`",
                    fired::FILE_CHANGED
                )));
            }
            Trigger::Event(_) => {}
        }
    }
    Ok(())
}

pub fn save(config: &Config, schedule: &Schedule) -> AppResult<()> {
    let settings = Settings {
        name: schedule.name.clone(),
        area: schedule.area.clone(),
        enabled: schedule.enabled,
        request: schedule.request.clone(),
        allowed_tools: schedule.allowed_tools.clone(),
        script: schedule.script.clone(),
        trigger: None,
        triggers: schedule.triggers.clone(),
    };
    let text = frontmatter::join(&toml::to_string(&settings)?, &schedule.prompt);
    std::fs::write(checked_path(config, &schedule.id)?, text)?;
    Ok(())
}

/// `base`, or `base-2`, `base-3`… if a schedule with that id already exists.
pub fn free_id(config: &Config, base: &str) -> String {
    (1_u32..=999)
        .map(|n| {
            if n == 1 {
                base.to_owned()
            } else {
                format!("{base}-{n}")
            }
        })
        .find(|id| !path(config, id).exists())
        .unwrap_or_else(|| base.to_owned())
}

/// A file-safe id from a name, e.g. "Morning brief" → "morning-brief".
pub fn slug(name: &str) -> String {
    let words: Vec<String> = name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    if words.is_empty() {
        "schedule".to_owned()
    } else {
        words.join("-")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_names() {
        assert_eq!(slug("End-of-day wrap!"), "end-of-day-wrap");
        assert_eq!(slug("  "), "schedule");
    }

    #[test]
    fn reads_old_single_trigger_files() {
        let front = "name = \"CI\"\narea = \"work\"\nscope = \"each_pr\"\nautonomy = \"ask\"\n\
                     enabled = true\n[trigger]\ntype = \"event\"\nkind = \"ci_failed\"\n";
        let mut s: Settings = toml::from_str(front).unwrap();
        let all = s.all_triggers();
        assert_eq!(all.len(), 1);
        assert!(matches!(&all[0], Trigger::Event(e) if e.on == "github.ci_failed"));
    }

    #[test]
    fn script_schedules_round_trip() {
        use crate::domain::script::{Report, Script};
        use crate::domain::trigger::OnEvent;
        let dir = std::env::temp_dir().join("quarterdeck-test-script-schedule");
        let config = Config::open(dir.clone()).unwrap();
        std::fs::create_dir_all(dir.join("schedules")).unwrap();
        let schedule = Schedule {
            id: "backup".to_owned(),
            name: "Backup".to_owned(),
            prompt: "restic backup ~/Documents\n".to_owned(),
            script: Some(Script {
                cwd: Some("~/code".to_owned()),
                timeout_minutes: 5,
                report: Report::Always,
            }),
            triggers: vec![
                Trigger::Cron {
                    expr: "0 2 * * *".to_owned(),
                },
                Trigger::Event(OnEvent {
                    on: "custom.deployed".to_owned(),
                    when: [("env".to_owned(), "prod".to_owned())].into(),
                    cooldown: 30,
                }),
            ],
            area: "work".to_owned(),
            enabled: true,
            request: String::new(),
            allowed_tools: Vec::new(),
        };
        save(&config, &schedule).unwrap();
        let back = find(&config, "backup").unwrap();
        assert_eq!(back.script, schedule.script);
        assert_eq!(back.triggers, schedule.triggers);
        assert_eq!(back.prompt, schedule.prompt.trim_end());
    }

    #[test]
    fn refuses_ids_that_are_not_slugs() {
        let config = Config::open(std::env::temp_dir().join("quarterdeck-test-config")).unwrap();
        assert!(checked_path(&config, "../quarterdeck.toml").is_err());
        assert!(checked_path(&config, "").is_err());
        assert!(checked_path(&config, "morning-brief").is_ok());
    }
}
