use actix_web::{HttpResponse, error::ErrorInternalServerError, post, web};

use crate::{AdminSession, Context, pipeline};

/// Runs one full background pass (ingest → extract → regenerate notifications)
/// on demand, under the same advisory lock the cron job uses, and records a
/// `sync_runs` row. Returns the pass report as JSON. Requires an authenticated
/// admin session.
#[post("/admin/pipeline")]
pub async fn post(
    ctx: web::Data<Context>,
    _admin: AdminSession,
) -> Result<HttpResponse, actix_web::Error> {
    let report = pipeline::run_once(&ctx, "manual")
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().json(report))
}
