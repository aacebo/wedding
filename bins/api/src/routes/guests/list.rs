use actix_web::{HttpResponse, get};

use crate::RequestContext;

#[get("/guests")]
pub async fn get(ctx: RequestContext) -> Result<HttpResponse, actix_web::Error> {
    let guests = ctx
        .storage()
        .guests()
        .list()
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(guests))
}
