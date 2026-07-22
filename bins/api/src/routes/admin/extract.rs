use actix_web::{HttpResponse, error::ErrorInternalServerError, error::ErrorNotFound, post, web};

use crate::{AdminSession, Context, extract};

/// Runs the OpenAI extraction pipeline over unprocessed communications and
/// returns per-run counts plus running totals. Requires an authenticated admin
/// session; 404s (staying hidden) when OpenAI isn't configured.
#[post("/admin/extract")]
pub async fn post(
    ctx: web::Data<Context>,
    _admin: AdminSession,
) -> Result<HttpResponse, actix_web::Error> {
    let extractor = ctx.llm().ok_or_else(|| ErrorNotFound("Not Found"))?;

    let report = extract::extract_pending(
        &ctx.storage(),
        extractor,
        extractor.model(),
        ctx.llm_max_batch(),
    )
    .await
    .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(report))
}
