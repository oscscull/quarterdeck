//! The user correcting the Briefing or adding context: a conversation with Claude
//! (`prompts/correct.md`) that saves references, agrees any item changes, then asks for a rewrite.
//! Nothing is stored: the window keeps the conversation, and what lasts lands as references,
//! records and item changes.

use schemars::JsonSchema;
use serde::Deserialize;

use super::view;
use crate::claude::{
    self,
    transcript::{self, ChatLine},
};
use crate::config::Config;
use crate::config::prompts::{Prompt, render};
use crate::config::settings::Settings;
use crate::error::AppResult;
use crate::store::Db;

/// One message in the correction box, with the conversation so far.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct Correction {
    /// What the user is looking at, in words, e.g. "the Approvals project".
    pub scope: String,
    /// The area shown, if one is chosen.
    pub area: Option<String>,
    /// The project shown, if any.
    pub project: Option<String>,
    #[serde(default)]
    pub history: Vec<ChatLine>,
    pub message: String,
}

/// The briefing the user is looking at, as the prompt shows it.
async fn context(db: &Db, settings: &Settings, c: &Correction) -> AppResult<String> {
    let mut shown = view::build(db, settings, c.area.as_deref()).await?;
    if let Some(project) = &c.project {
        shown.projects.retain(|p| &p.project.id == project);
        shown.areas.clear();
    }
    Ok(serde_json::to_string_pretty(&shown)?)
}

/// Claude's reply to one message.
pub async fn reply(
    db: &Db,
    config: &Config,
    settings: &Settings,
    c: &Correction,
) -> AppResult<String> {
    let shown = context(db, settings, c).await?;
    let history = transcript::render(&c.history, settings.chat_history_messages);
    let vars = [
        ("scope", c.scope.as_str()),
        ("context", shown.as_str()),
        ("history", history.as_str()),
        ("message", c.message.as_str()),
    ];
    let prompt = render(config, Prompt::Correct, &vars)?;
    claude::ask(&settings.claude()?, &prompt).await
}
