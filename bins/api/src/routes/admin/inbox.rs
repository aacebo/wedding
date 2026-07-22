use actix_web::{Error, error::ErrorInternalServerError, get, web, web::Html};
use askama::Template;
use serde::Deserialize;

use storage::types::CommSource;

use crate::{AdminSession, Context};

#[derive(Template)]
#[template(path = "admin/inbox.html")]
struct InboxPage {
    sources: Vec<CommSource>,
    selected: Option<String>,
    selected_is_gmail: bool,
    selected_is_drive: bool,
}

#[derive(Deserialize)]
pub struct InboxQuery {
    provider: Option<String>,
}

#[get("/admin/inbox")]
pub async fn get(
    ctx: web::Data<Context>,
    _admin: AdminSession,
    q: web::Query<InboxQuery>,
) -> Result<Html, Error> {
    // Only gmail/drive are valid providers; anything else falls back to "all".
    let selected = q
        .provider
        .as_deref()
        .filter(|p| *p == "gmail" || *p == "drive")
        .map(|p| p.to_string());

    let sources = ctx
        .storage()
        .comm_sources()
        .recent(selected.as_deref(), 100)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(Html::new(
        InboxPage {
            selected_is_gmail: selected.as_deref() == Some("gmail"),
            selected_is_drive: selected.as_deref() == Some("drive"),
            selected,
            sources,
        }
        .render()
        .map_err(ErrorInternalServerError)?,
    ))
}
