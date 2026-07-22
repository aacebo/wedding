use actix_web::{HttpResponse, error::ErrorInternalServerError, error::ErrorNotFound, post, web};

use crate::{AdminSession, Context, ingest};

/// Runs Gmail + Drive ingestion for all linked accounts and returns per-account
/// new/updated/skipped counts. Requires an authenticated admin session; 404s
/// (staying hidden) when Google SSO isn't configured.
#[post("/admin/sync")]
pub async fn post(
    ctx: web::Data<Context>,
    _admin: AdminSession,
) -> Result<HttpResponse, actix_web::Error> {
    let google = ctx.google().ok_or_else(|| ErrorNotFound("Not Found"))?;

    let report = ingest::sync_all(&ctx.storage(), google)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(report))
}
