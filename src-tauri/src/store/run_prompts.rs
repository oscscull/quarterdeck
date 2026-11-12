//! What an agent run about an item was asked, kept with its run so a failed one can be asked again.

use super::client;
use super::convert::key;
use super::db::Db;
use crate::error::{AppError, AppResult};

/// A run to ask again: what it was for, its item, and its prompt.
#[derive(Debug, Clone)]
pub struct Asked {
    pub what: String,
    pub item_id: String,
    pub prompt: String,
}

pub async fn keep(db: &Db, run: i64, prompt: &str) -> AppResult<()> {
    let input = client::SetRunPromptInput {
        id: key(run),
        prompt: prompt.to_owned(),
    };
    Ok(db.api().set_run_prompt(input, ()).await?)
}

/// Fails when the run has expired or never kept its prompt, e.g. a schedule's run.
pub async fn get(db: &Db, run: i64) -> AppResult<Asked> {
    let row = db
        .api()
        .run_retry(client::RunRetryInput { id: key(run) }, ())
        .await?;
    let gone = || AppError::NotFound(format!("run {run} to retry; it may have expired"));
    let row = row.ok_or_else(gone)?;
    Ok(Asked {
        what: row.what,
        item_id: row.target,
        prompt: row.prompt.ok_or_else(gone)?,
    })
}
