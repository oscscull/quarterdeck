use chrono::{NaiveDate, Utc};
use tauri::State;

use crate::app::App;
use crate::domain::todo::{Todo, link_or_none};
use crate::error::AppResult;
use crate::store::todos;

/// What the add form sends.
#[derive(Debug, serde::Deserialize)]
pub struct NewTodo {
    title: String,
    area: String,
    due: Option<NaiveDate>,
    link: Option<String>,
}

#[tauri::command]
pub async fn add_todo(app: State<'_, App>, todo: NewTodo) -> AppResult<()> {
    let todo = Todo {
        id: 0,
        title: todo.title,
        area: app.settings()?.known_area(&todo.area),
        due: todo.due,
        link: link_or_none(todo.link),
        done_at: None,
    };
    todos::insert(&app.db, &todo).await
}

/// Ticking a to-do off keeps it briefly, so a mis-click can be undone; expiry removes it later.
#[tauri::command]
pub async fn set_todo_done(app: State<'_, App>, id: i64, done: bool) -> AppResult<()> {
    let at = done.then(Utc::now);
    todos::set_done(&app.db, id, at).await
}

/// What the edit form sends: the whole to-do, so clearing the date clears it.
#[derive(Debug, serde::Deserialize)]
pub struct TodoEdit {
    id: i64,
    title: String,
    area: String,
    due: Option<NaiveDate>,
    link: Option<String>,
}

#[tauri::command]
pub async fn update_todo(app: State<'_, App>, todo: TodoEdit) -> AppResult<()> {
    let area = app.settings()?.known_area(&todo.area);
    let old = todos::get(&app.db, todo.id).await?;
    let new = Todo {
        title: todo.title,
        area,
        due: todo.due,
        link: link_or_none(todo.link),
        ..old
    };
    todos::update(&app.db, &new).await
}

#[tauri::command]
pub async fn delete_todo(app: State<'_, App>, id: i64) -> AppResult<()> {
    todos::delete(&app.db, id).await
}
