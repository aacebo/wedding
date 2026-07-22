use actix_web::{
    Error, error::ErrorInternalServerError, get, patch, post, web, web::Html,
};
use askama::Template;
use serde::Deserialize;

use storage::Storage;
use storage::types::Notification;

use crate::{AdminSession, Context};

#[derive(Template)]
#[template(path = "admin/_notifications.html")]
struct NotificationsPanel {
    notifications: Vec<Notification>,
    unread: i64,
}

#[derive(Template)]
#[template(path = "admin/_notif_badge.html")]
struct NotifBadge {
    unread: i64,
}

#[derive(Deserialize)]
struct StatusQuery {
    value: String,
}

/// Renders the notifications panel fragment after any change. The fragment
/// carries an out-of-band badge span so the nav count updates in the same swap.
async fn render_panel(storage: &Storage<'_>) -> Result<Html, Error> {
    let notifs = storage.notifications();
    let notifications = notifs.list().await.map_err(ErrorInternalServerError)?;
    let unread = notifs.unread_count().await.map_err(ErrorInternalServerError)?;
    Ok(Html::new(
        NotificationsPanel {
            notifications,
            unread,
        }
        .render()
        .map_err(ErrorInternalServerError)?,
    ))
}

/// Renders the self-refreshing badge fragment (also re-arms htmx polling).
async fn render_badge(storage: &Storage<'_>) -> Result<Html, Error> {
    let unread = storage
        .notifications()
        .unread_count()
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(Html::new(
        NotifBadge { unread }
            .render()
            .map_err(ErrorInternalServerError)?,
    ))
}

#[get("/admin/notifications")]
pub async fn panel(ctx: web::Data<Context>, _admin: AdminSession) -> Result<Html, Error> {
    render_panel(&ctx.storage()).await
}

#[get("/admin/notifications/count")]
pub async fn count(ctx: web::Data<Context>, _admin: AdminSession) -> Result<Html, Error> {
    render_badge(&ctx.storage()).await
}

#[patch("/admin/notifications/{id}")]
pub async fn set_status(
    ctx: web::Data<Context>,
    _admin: AdminSession,
    id: web::Path<uuid::Uuid>,
    q: web::Query<StatusQuery>,
) -> Result<Html, Error> {
    let status = match q.value.as_str() {
        "read" => "read",
        "unread" => "unread",
        "dismissed" => "dismissed",
        _ => return Err(actix_web::error::ErrorBadRequest("invalid status")),
    };
    let storage = ctx.storage();
    storage
        .notifications()
        .set_status(*id, status)
        .await
        .map_err(ErrorInternalServerError)?;
    render_panel(&storage).await
}

#[post("/admin/notifications/read-all")]
pub async fn read_all(ctx: web::Data<Context>, _admin: AdminSession) -> Result<Html, Error> {
    let storage = ctx.storage();
    storage
        .notifications()
        .mark_all_read()
        .await
        .map_err(ErrorInternalServerError)?;
    render_panel(&storage).await
}

#[post("/admin/notifications/refresh")]
pub async fn refresh(ctx: web::Data<Context>, _admin: AdminSession) -> Result<Html, Error> {
    let storage = ctx.storage();
    storage
        .notifications()
        .generate()
        .await
        .map_err(ErrorInternalServerError)?;
    render_panel(&storage).await
}
