use actix_web::{
    Error, delete, error::ErrorInternalServerError, get, patch, post, web, web::Html,
};
use askama::Template;
use serde::Deserialize;

use storage::Storage;
use storage::types::{NewAdminTodo, TodoRow};

use crate::{AdminSession, Context};

#[derive(Template)]
#[template(path = "admin/todos.html")]
struct TodosPage {
    todos: Vec<TodoRow>,
}

#[derive(Template)]
#[template(path = "admin/_todos_list.html")]
struct TodosList {
    todos: Vec<TodoRow>,
}

/// Fields shared by the add-todo and edit-todo forms. All values arrive as
/// strings; empty ones are normalized to `None` / dropped.
#[derive(Deserialize)]
struct TodoForm {
    title: String,
    owner: Option<String>,
    due_date: Option<String>,
    priority: Option<String>,
    notes: Option<String>,
}

#[derive(Deserialize)]
struct StatusQuery {
    value: String,
}

fn norm(s: Option<String>) -> Option<String> {
    s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

fn parse_date(s: Option<String>) -> Option<chrono::NaiveDate> {
    let s = norm(s)?;
    chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()
}

fn priority(s: Option<String>) -> String {
    match norm(s).as_deref() {
        Some("low") => "low",
        Some("high") => "high",
        _ => "med",
    }
    .to_string()
}

/// Re-renders the todos list partial for htmx swaps after a mutation.
async fn render_list(storage: &Storage<'_>) -> Result<Html, Error> {
    let todos = storage
        .admin_items()
        .list_todos()
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(Html::new(
        TodosList { todos }.render().map_err(ErrorInternalServerError)?,
    ))
}

#[get("/admin/todos")]
pub async fn get(ctx: web::Data<Context>, _admin: AdminSession) -> Result<Html, Error> {
    let todos = ctx
        .storage()
        .admin_items()
        .list_todos()
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(Html::new(
        TodosPage { todos }.render().map_err(ErrorInternalServerError)?,
    ))
}

#[post("/admin/todos")]
pub async fn create(
    ctx: web::Data<Context>,
    _admin: AdminSession,
    form: web::Form<TodoForm>,
) -> Result<Html, Error> {
    let form = form.into_inner();
    let storage = ctx.storage();
    storage
        .admin_items()
        .insert_todo(
            &NewAdminTodo {
                source_id: None,
                title: form.title.trim().to_string(),
                owner: norm(form.owner),
                due_date: parse_date(form.due_date),
                priority: priority(form.priority),
                confidence: 1.0,
                notes: norm(form.notes),
            },
            "human",
        )
        .await
        .map_err(ErrorInternalServerError)?;
    render_list(&storage).await
}

#[patch("/admin/todos/{id}")]
pub async fn update(
    ctx: web::Data<Context>,
    _admin: AdminSession,
    id: web::Path<uuid::Uuid>,
    form: web::Form<TodoForm>,
) -> Result<Html, Error> {
    let form = form.into_inner();
    let storage = ctx.storage();
    storage
        .admin_items()
        .update_todo(
            *id,
            form.title.trim(),
            norm(form.owner).as_deref(),
            parse_date(form.due_date),
            &priority(form.priority),
            norm(form.notes).as_deref(),
        )
        .await
        .map_err(ErrorInternalServerError)?;
    render_list(&storage).await
}

#[patch("/admin/todos/{id}/status")]
pub async fn set_status(
    ctx: web::Data<Context>,
    _admin: AdminSession,
    id: web::Path<uuid::Uuid>,
    q: web::Query<StatusQuery>,
) -> Result<Html, Error> {
    let status = match q.value.as_str() {
        "done" => "done",
        "dismissed" => "dismissed",
        "open" => "open",
        _ => return Err(actix_web::error::ErrorBadRequest("invalid status")),
    };
    let storage = ctx.storage();
    storage
        .admin_items()
        .set_todo_status(*id, status)
        .await
        .map_err(ErrorInternalServerError)?;
    render_list(&storage).await
}

#[delete("/admin/todos/{id}")]
pub async fn remove(
    ctx: web::Data<Context>,
    _admin: AdminSession,
    id: web::Path<uuid::Uuid>,
) -> Result<Html, Error> {
    let storage = ctx.storage();
    storage
        .admin_items()
        .delete_todo(*id)
        .await
        .map_err(ErrorInternalServerError)?;
    render_list(&storage).await
}
