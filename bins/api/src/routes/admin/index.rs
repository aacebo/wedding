use actix_web::{Error, error::ErrorInternalServerError, get, web, web::Html};
use askama::Template;

use storage::types::{CommSource, DeadlineRow, SyncRun};

use crate::{AdminSession, Context};

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
struct Dashboard {
    email: String,
    google_enabled: bool,
    google_linked: bool,
    llm_enabled: bool,
    open_todos: i64,
    event_count: i64,
    deadline_count: i64,
    upcoming: Vec<DeadlineRow>,
    recent: Vec<CommSource>,
    last_run: Option<SyncRun>,
}

#[get("/admin")]
pub async fn get(ctx: web::Data<Context>, admin: AdminSession) -> Result<Html, Error> {
    let storage = ctx.storage();

    let google_enabled = ctx.google().is_some();
    let google_linked = if google_enabled {
        storage
            .google()
            .get(&admin.email)
            .await
            .map_err(ErrorInternalServerError)?
            .is_some()
    } else {
        false
    };

    let admin_items = storage.admin_items();
    let open_todos = admin_items
        .open_todo_count()
        .await
        .map_err(ErrorInternalServerError)?;
    let (_todos, event_count, deadline_count) =
        admin_items.counts().await.map_err(ErrorInternalServerError)?;
    let upcoming = admin_items
        .upcoming_deadlines(5)
        .await
        .map_err(ErrorInternalServerError)?;
    let recent = storage
        .comm_sources()
        .recent(None, 5)
        .await
        .map_err(ErrorInternalServerError)?;
    let last_run = storage
        .sync_runs()
        .latest()
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(Html::new(
        Dashboard {
            email: admin.email,
            google_enabled,
            google_linked,
            llm_enabled: ctx.llm().is_some(),
            open_todos,
            event_count,
            deadline_count,
            upcoming,
            recent,
            last_run,
        }
        .render()
        .map_err(ErrorInternalServerError)?,
    ))
}
