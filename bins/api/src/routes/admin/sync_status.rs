use actix_web::{HttpResponse, error::ErrorInternalServerError, get, web};

use crate::{AdminSession, Context};

/// Reports the last sync time and cursor for each linked account/provider, plus
/// the total number of ingested sources. Requires an authenticated admin session.
#[get("/admin/sync/status")]
pub async fn get(
    ctx: web::Data<Context>,
    _admin: AdminSession,
) -> Result<HttpResponse, actix_web::Error> {
    let storage = ctx.storage();
    let states = storage
        .sync_state()
        .list()
        .await
        .map_err(ErrorInternalServerError)?;
    let total_sources = storage
        .comm_sources()
        .count()
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "google_enabled": ctx.google().is_some(),
        "total_sources": total_sources,
        "sync_state": states,
    })))
}
