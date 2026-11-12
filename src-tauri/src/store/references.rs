use std::collections::HashMap;

use chrono::{DateTime, Utc};

use super::client::{self, RefRow};
use super::convert::{int, key, same, text_key};
use super::db::{Db, gone_ok};
use super::time;
use crate::domain::reference::{RefBy, RefKind, Reference};
use crate::error::{AppError, AppResult};

/// Which items each reference is about.
async fn links(db: &Db) -> AppResult<HashMap<i64, Vec<String>>> {
    let rows = db
        .api()
        .reference_links(client::ReferenceLinksInput {}, ())
        .await?;
    let mut map: HashMap<i64, Vec<String>> = HashMap::new();
    for row in rows {
        map.entry(int(&row.reference_id)?)
            .or_default()
            .push(row.item_id.into_raw());
    }
    Ok(map)
}

fn from_row(row: RefRow, links: &mut HashMap<i64, Vec<String>>) -> AppResult<Reference> {
    let id = int(&row.id)?;
    Ok(Reference {
        id,
        area: row.area,
        kind: same(&row.kind)?,
        title: row.title,
        url: row.url,
        quote: row.quote,
        by: same(&row.by)?,
        at: time::get(&row.at)?,
        project: row.project.map(|p| p.id.into_raw()),
        items: links.remove(&id).unwrap_or_default(),
    })
}

/// One area's references, newest first.
pub async fn in_area(db: &Db, area: &str) -> AppResult<Vec<Reference>> {
    let input = client::ReferencesInInput {
        area: area.to_owned(),
    };
    let rows = db.api().references_in(input, ()).await?;
    let mut links = links(db).await?;
    rows.into_iter().map(|r| from_row(r, &mut links)).collect()
}

pub async fn get(db: &Db, id: i64) -> AppResult<Reference> {
    let input = client::ReferenceInput { id: key(id) };
    let row = db
        .api()
        .reference(input, ())
        .await?
        .ok_or_else(|| AppError::NotFound(format!("reference {id}")))?;
    from_row(row, &mut links(db).await?)
}

/// What a reference says, as an agent or the UI sends it.
#[derive(Debug, Clone)]
pub struct Source {
    pub kind: RefKind,
    pub title: String,
    pub url: Option<String>,
    pub quote: String,
    pub project: Option<String>,
    pub items: Vec<String>,
}

/// Stores a reference and links it to its items. Returns its id.
pub async fn add(
    db: &Db,
    (area, by, at): (&str, RefBy, DateTime<Utc>),
    source: &Source,
) -> AppResult<i64> {
    let input = client::AddReferenceInput {
        area: area.to_owned(),
        kind: same(&source.kind)?,
        title: source.title.clone(),
        url: source.url.clone(),
        quote: source.quote.clone(),
        by: same(&by)?,
        at: time::put(at),
        project: source.project.as_deref().map(text_key),
    };
    let row = db.api().add_reference(input, ()).await?;
    let id = int(&row.id)?;
    relink(db, id, &source.items).await?;
    Ok(id)
}

/// Rewrites a reference, links included.
pub async fn update(db: &Db, id: i64, source: &Source) -> AppResult<()> {
    let input = client::UpdateReferenceInput {
        id: key(id),
        kind: same(&source.kind)?,
        title: source.title.clone(),
        url: source.url.clone(),
        quote: source.quote.clone(),
        project: source.project.as_deref().map(text_key),
    };
    db.api().update_reference(input, ()).await?;
    relink(db, id, &source.items).await
}

/// Points a reference at exactly `items`.
async fn relink(db: &Db, id: i64, items: &[String]) -> AppResult<()> {
    let api = db.api();
    let clear = client::ClearReferenceLinksInput { reference: key(id) };
    api.clear_reference_links(clear, ()).await?;
    let mut seen: Vec<&String> = Vec::new();
    for item in items {
        if seen.contains(&item) {
            continue;
        }
        seen.push(item);
        let link = client::LinkReferenceInput {
            reference: key(id),
            item: text_key(item),
        };
        api.link_reference(link, ()).await?;
    }
    Ok(())
}

pub async fn remove(db: &Db, id: i64) -> AppResult<()> {
    let input = client::RemoveReferenceInput { id: key(id) };
    gone_ok(
        db.api()
            .remove_reference(input, ())
            .await
            .map_err(AppError::from),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::item::{ItemInput, Stage};

    fn source(project: Option<&str>, items: &[&str]) -> Source {
        Source {
            kind: RefKind::YouSaid,
            title: "Blocked on AWS".to_owned(),
            url: None,
            quote: "waiting for AWS access".to_owned(),
            project: project.map(str::to_owned),
            items: items.iter().map(|i| (*i).to_owned()).collect(),
        }
    }

    async fn item(db: &Db, id: &str) {
        let input = ItemInput {
            id: id.to_owned(),
            title: id.to_owned(),
            state_label: String::new(),
            stage: Stage::UpNext,
            status: None,
            area: "work".to_owned(),
            link: None,
        };
        crate::work::save(db, input, Utc::now()).await.unwrap();
    }

    #[tokio::test]
    async fn lists_references_with_and_without_a_project_and_their_items() {
        let db = Db::in_memory().await;
        item(&db, "A-1").await;
        item(&db, "A-2").await;
        let project = crate::domain::project::Project {
            id: "approvals".to_owned(),
            title: "Approvals".to_owned(),
            aim: "Ship it".to_owned(),
            area: "work".to_owned(),
            state: crate::domain::project::ProjectState::Moving,
            rank: 0,
            opened_at: Utc::now(),
            state_since: Utc::now(),
            briefing: None,
            briefed_at: None,
        };
        super::super::projects::save(&db, &project).await.unwrap();
        let at = ("work", RefBy::You, Utc::now());
        let loose = add(&db, at, &source(None, &["A-1", "A-1"])).await.unwrap();
        let grouped = add(&db, at, &source(Some("approvals"), &["A-1", "A-2"]))
            .await
            .unwrap();
        let all = in_area(&db, "work").await.unwrap();
        assert_eq!(all.len(), 2);
        let loose = all.iter().find(|r| r.id == loose).unwrap();
        assert_eq!((loose.project.as_deref(), loose.items.len()), (None, 1));
        let grouped = all.iter().find(|r| r.id == grouped).unwrap();
        assert_eq!(grouped.project.as_deref(), Some("approvals"));
        assert_eq!(grouped.items.len(), 2);

        super::super::projects::delete(&db, "approvals")
            .await
            .unwrap();
        assert_eq!(
            in_area(&db, "work").await.unwrap().len(),
            1,
            "goes with its project"
        );
    }
}
