use chrono::{DateTime, Utc};

use super::client::{self, TodoRow};
use super::convert::{int, key};
use super::db::{Db, gone_ok};
use super::time;
use crate::domain::todo::Todo;
use crate::error::{AppError, AppResult};

fn from_row(row: TodoRow) -> AppResult<Todo> {
    Ok(Todo {
        id: int(&row.id)?,
        title: row.title,
        area: row.area,
        due: row.due.as_deref().map(time::get_date).transpose()?,
        link: row.link,
        done_at: time::get_opt(row.done_at.as_deref())?,
    })
}

/// Open to-dos (soonest due first, undated last), then done ones, latest first.
pub async fn list(db: &Db) -> AppResult<Vec<Todo>> {
    let rows = db.api().todos(client::TodosInput {}, ()).await?;
    rows.into_iter().map(from_row).collect()
}

/// Stores a new to-do; its `id` is ignored and assigned by the database.
pub async fn insert(db: &Db, todo: &Todo) -> AppResult<()> {
    let input = client::AddTodoInput {
        title: todo.title.clone(),
        area: todo.area.clone(),
        due: todo.due.map(time::put_date),
        link: todo.link.clone(),
        done_at: time::put_opt(todo.done_at),
    };
    Ok(db.api().add_todo(input, ()).await?)
}

pub async fn set_done(db: &Db, id: i64, done_at: Option<DateTime<Utc>>) -> AppResult<()> {
    let input = client::SetTodoDoneInput {
        id: key(id),
        done_at: time::put_opt(done_at),
    };
    Ok(db.api().set_todo_done(input, ()).await?)
}

pub async fn get(db: &Db, id: i64) -> AppResult<Todo> {
    let input = client::TodoInput { id: key(id) };
    let row = db.api().todo(input, ()).await?;
    from_row(row.ok_or_else(|| AppError::NotFound(format!("to-do {id}")))?)
}

pub async fn update(db: &Db, todo: &Todo) -> AppResult<()> {
    let input = client::UpdateTodoInput {
        id: key(todo.id),
        title: todo.title.clone(),
        area: todo.area.clone(),
        due: todo.due.map(time::put_date),
        link: todo.link.clone(),
        done_at: time::put_opt(todo.done_at),
    };
    Ok(db.api().update_todo(input, ()).await?)
}

pub async fn delete(db: &Db, id: i64) -> AppResult<()> {
    let input = client::DeleteTodoInput { id: key(id) };
    gone_ok(
        db.api()
            .delete_todo(input, ())
            .await
            .map_err(AppError::from),
    )
}
