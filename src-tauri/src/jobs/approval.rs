//! A script runs only once the user has approved exactly that script, in that folder. Agents can
//! write schedule files and call every tool, so approving is only offered in the app's own window
//! and to peers; any change to the script or its folder needs approving again.

use sha2::{Digest as _, Sha256};

use crate::domain::schedule::Schedule;
use crate::error::AppResult;
use crate::store::{Db, meta};

fn key(schedule_id: &str) -> String {
    format!("script_approval.{schedule_id}")
}

/// What approval covers: the script and where it runs.
fn fingerprint(schedule: &Schedule) -> String {
    let cwd = schedule
        .script
        .as_ref()
        .and_then(|s| s.cwd.as_deref())
        .unwrap_or_default();
    let digest = Sha256::new()
        .chain_update(schedule.prompt.as_bytes())
        .chain_update([0])
        .chain_update(cwd.as_bytes())
        .finalize();
    digest
        .iter()
        .flat_map(|b| [b >> 4, b & 0xf])
        .filter_map(|nibble| char::from_digit(u32::from(nibble), 16))
        .collect()
}

pub async fn approve(db: &Db, schedule: &Schedule) -> AppResult<()> {
    meta::set_text(db, &key(&schedule.id), &fingerprint(schedule)).await
}

/// True for a Claude schedule, or a script approved as it stands.
pub async fn approved(db: &Db, schedule: &Schedule) -> AppResult<bool> {
    if schedule.script.is_none() {
        return Ok(true);
    }
    let stored = meta::text(db, &key(&schedule.id)).await?;
    Ok(stored.as_deref() == Some(fingerprint(schedule).as_str()))
}

pub async fn forget(db: &Db, schedule_id: &str) -> AppResult<()> {
    meta::forget(db, &key(schedule_id)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::script::Script;

    fn script(body: &str, cwd: Option<&str>) -> Schedule {
        Schedule {
            id: "backup".to_owned(),
            name: "Backup".to_owned(),
            prompt: body.to_owned(),
            script: Some(Script {
                cwd: cwd.map(str::to_owned),
                timeout_minutes: 10,
                report: crate::domain::script::Report::Failures,
            }),
            triggers: Vec::new(),
            area: "work".to_owned(),
            enabled: true,
            request: String::new(),
            allowed_tools: Vec::new(),
        }
    }

    #[tokio::test]
    async fn any_change_needs_approving_again() {
        let db = Db::in_memory().await;
        let first = script("echo hi", None);
        assert!(!approved(&db, &first).await.unwrap());
        approve(&db, &first).await.unwrap();
        assert!(approved(&db, &first).await.unwrap());
        assert!(!approved(&db, &script("rm -rf ~", None)).await.unwrap());
        assert!(
            !approved(&db, &script("echo hi", Some("/tmp")))
                .await
                .unwrap()
        );
    }
}
