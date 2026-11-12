//! Turns Claude's JSON reply from `prompts/draft-schedule.md` into a schedule.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::config::schedules::{check, slug};
use crate::domain::schedule::Schedule;
use crate::domain::trigger::{OnEvent, Trigger};
use crate::error::{AppError, AppResult};

/// `cron` and `event` are how prompts edited before triggers became a list still answer.
#[derive(Debug, Deserialize)]
struct Draft {
    name: String,
    area: String,
    #[serde(default)]
    triggers: Vec<Trigger>,
    cron: Option<String>,
    event: Option<String>,
    prompt: String,
}

impl Draft {
    fn all_triggers(&mut self) -> Vec<Trigger> {
        let cron = self.cron.take().map(|expr| Trigger::Cron { expr });
        let event = self.event.take().map(|on| {
            Trigger::Event(OnEvent {
                on,
                when: BTreeMap::new(),
                cooldown: 0,
            })
        });
        std::mem::take(&mut self.triggers)
            .into_iter()
            .chain(cron)
            .chain(event)
            .map(Trigger::normalised)
            .collect()
    }
}

/// The JSON object inside a reply, ignoring any text around it.
fn json_in(reply: &str) -> AppResult<&str> {
    let start = reply.find('{');
    let end = reply.rfind('}');
    start
        .zip(end)
        .and_then(|(s, e)| reply.get(s..=e))
        .ok_or_else(|| AppError::Claude("the reply had no JSON in it".to_owned()))
}

/// Builds a new, enabled schedule whose id is a slug of its name.
/// The caller checks the area against the configured ones.
pub fn parse(reply: &str, request: &str) -> AppResult<Schedule> {
    let mut draft: Draft = serde_json::from_str(json_in(reply)?)?;
    let triggers = draft.all_triggers();
    if triggers.is_empty() {
        return Err(AppError::Claude("the reply had no timing".to_owned()));
    }
    let schedule = Schedule {
        id: slug(&draft.name),
        triggers,
        name: draft.name,
        prompt: draft.prompt,
        script: None,
        area: draft.area,
        enabled: true,
        request: request.to_owned(),
        allowed_tools: Vec::new(),
    };
    check(&schedule)?;
    Ok(schedule)
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPLY: &str = r#"Sure: {"name": "CI watch", "area": "work",
        "triggers": [{"type": "event", "on": "github.ci_failed", "when": {"pr": "me/*"}}],
        "prompt": "Re-run flaky jobs."} done"#;

    #[test]
    fn parses_event_drafts_inside_chatter() {
        let s = parse(REPLY, "watch CI").unwrap();
        let [Trigger::Event(e)] = s.triggers.as_slice() else {
            panic!("expected one event trigger")
        };
        assert_eq!(e.on, "github.ci_failed");
        assert_eq!(e.when["pr"], "me/*");
        assert_eq!(s.request, "watch CI");
        assert_eq!(s.id, "ci-watch");
    }

    #[test]
    fn rejects_replies_without_json_or_timing() {
        assert!(parse("no idea", "").is_err());
        assert!(parse(r#"{"name":"x","area":"w","triggers":[],"prompt":""}"#, "").is_err());
    }

    #[test]
    fn reads_replies_from_older_prompts() {
        let reply = r#"{"name":"x","area":"w","cron":null,"event":"ci_failed","prompt":""}"#;
        let s = parse(reply, "").unwrap();
        assert!(matches!(&s.triggers[..], [Trigger::Event(e)] if e.on == "github.ci_failed"));
    }

    #[test]
    fn rejects_unknown_events() {
        let reply =
            r#"{"name":"x","area":"w","triggers":[{"type":"event","on":"made.up"}],"prompt":""}"#;
        assert!(parse(reply, "").is_err());
    }
}
