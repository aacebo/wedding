use actix_web::{HttpResponse, get};

use crate::RequestContext;

#[get("/guests")]
pub async fn get(ctx: RequestContext) -> Result<HttpResponse, actix_web::Error> {
    let guests = ctx.storage().guests().list().await.map_err(|error| {
        eprintln!("Guest list storage error [{}]: {error}", ctx.request_id());
        actix_web::error::ErrorInternalServerError("Unable to load guests.")
    })?;

    Ok(HttpResponse::Ok().json(guests))
}
