use actix_web::{Error, error::ErrorInternalServerError, delete, get, patch, post, web, web::Html};
use askama::Template;
use serde::Deserialize;

use storage::Storage;
use storage::types::{NewDeadline, NewTimelineEvent};

use crate::{AdminSession, Context};

/// A unified timeline entry (event or deadline) for chronological display.
struct TimelineRow {
    id: uuid::Uuid,
    kind: String,
    title: String,
    date: Option<chrono::NaiveDate>,
    tag: String,
    status: String,
    needs_review: bool,
    source_title: Option<String>,
    source_url: Option<String>,
}

#[derive(Template)]
#[template(path = "admin/timeline.html")]
struct TimelinePage {
    rows: Vec<TimelineRow>,
}

#[derive(Template)]
#[template(path = "admin/_timeline_list.html")]
struct TimelineList {
    rows: Vec<TimelineRow>,
}

#[derive(Deserialize)]
struct EventForm {
    title: String,
    event_date: Option<String>,
    category: Option<String>,
}

#[derive(Deserialize)]
struct DeadlineForm {
    title: String,
    due_date: Option<String>,
    severity: Option<String>,
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

fn severity(s: Option<String>) -> String {
    match norm(s).as_deref() {
        Some("low") => "low",
        Some("high") => "high",
        _ => "med",
    }
    .to_string()
}

/// Loads events + deadlines and merges them into one date-sorted list.
async fn build_rows(storage: &Storage<'_>) -> Result<Vec<TimelineRow>, Error> {
    let items = storage.admin_items();
    let events = items.list_events().await.map_err(ErrorInternalServerError)?;
    let deadlines = items
        .list_deadlines()
        .await
        .map_err(ErrorInternalServerError)?;

    let mut rows: Vec<TimelineRow> = Vec::with_capacity(events.len() + deadlines.len());
    for e in events {
        rows.push(TimelineRow {
            id: e.id,
            kind: "event".to_string(),
            title: e.title,
            date: e.event_date,
            tag: e.category.unwrap_or_default(),
            status: "open".to_string(),
            needs_review: e.created_by == "ai" && e.confidence < 0.5,
            source_title: e.source_title,
            source_url: e.source_url,
        });
    }
    for d in deadlines {
        let needs_review = d.created_by == "ai" && d.confidence < 0.5;
        rows.push(TimelineRow {
            id: d.id,
            kind: "deadline".to_string(),
            title: d.title,
            date: d.due_date,
            tag: d.severity,
            status: d.status,
            needs_review,
            source_title: d.source_title,
            source_url: d.source_url,
        });
    }
    // Chronological, undated entries last.
    rows.sort_by(|a, b| match (a.date, b.date) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    Ok(rows)
}

async fn render_list(storage: &Storage<'_>) -> Result<Html, Error> {
    let rows = build_rows(storage).await?;
    Ok(Html::new(
        TimelineList { rows }
            .render()
            .map_err(ErrorInternalServerError)?,
    ))
}

#[get("/admin/timeline")]
pub async fn get(ctx: web::Data<Context>, _admin: AdminSession) -> Result<Html, Error> {
    let storage = ctx.storage();
    let rows = build_rows(&storage).await?;
    Ok(Html::new(
        TimelinePage { rows }
            .render()
            .map_err(ErrorInternalServerError)?,
    ))
}

#[post("/admin/events")]
pub async fn create_event(
    ctx: web::Data<Context>,
    _admin: AdminSession,
    form: web::Form<EventForm>,
) -> Result<Html, Error> {
    let form = form.into_inner();
    let storage = ctx.storage();
    storage
        .admin_items()
        .insert_event(
            &NewTimelineEvent {
                source_id: None,
                title: form.title.trim().to_string(),
                event_date: parse_date(form.event_date),
                category: norm(form.category),
                confidence: 1.0,
            },
            "human",
        )
        .await
        .map_err(ErrorInternalServerError)?;
    render_list(&storage).await
}

#[post("/admin/deadlines")]
pub async fn create_deadline(
    ctx: web::Data<Context>,
    _admin: AdminSession,
    form: web::Form<DeadlineForm>,
) -> Result<Html, Error> {
    let form = form.into_inner();
    let storage = ctx.storage();
    storage
        .admin_items()
        .insert_deadline(
            &NewDeadline {
                source_id: None,
                title: form.title.trim().to_string(),
                due_date: parse_date(form.due_date),
                severity: severity(form.severity),
                confidence: 1.0,
            },
            "human",
        )
        .await
        .map_err(ErrorInternalServerError)?;
    render_list(&storage).await
}

#[patch("/admin/deadlines/{id}/status")]
pub async fn set_deadline_status(
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
        .set_deadline_status(*id, status)
        .await
        .map_err(ErrorInternalServerError)?;
    render_list(&storage).await
}

#[delete("/admin/deadlines/{id}")]
pub async fn remove_deadline(
    ctx: web::Data<Context>,
    _admin: AdminSession,
    id: web::Path<uuid::Uuid>,
) -> Result<Html, Error> {
    let storage = ctx.storage();
    storage
        .admin_items()
        .delete_deadline(*id)
        .await
        .map_err(ErrorInternalServerError)?;
    render_list(&storage).await
}

#[delete("/admin/events/{id}")]
pub async fn remove_event(
    ctx: web::Data<Context>,
    _admin: AdminSession,
    id: web::Path<uuid::Uuid>,
) -> Result<Html, Error> {
    let storage = ctx.storage();
    storage
        .admin_items()
        .delete_event(*id)
        .await
        .map_err(ErrorInternalServerError)?;
    render_list(&storage).await
}
