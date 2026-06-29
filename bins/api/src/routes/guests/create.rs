use actix_web::{HttpResponse, post, web};
use storage::types::Guest;

use crate::RequestContext;

#[derive(Debug, serde::Deserialize)]
struct CreateGuest {
    pub name: String,
    pub email: String,
    pub plus_one: bool,
}

#[post("/guests")]
pub async fn post(
    ctx: RequestContext,
    body: web::Json<CreateGuest>,
) -> Result<HttpResponse, actix_web::Error> {
    let guest = Guest::new(&body.name, &body.email, body.plus_one);

    let guest = ctx
        .storage()
        .guests()
        .create(&guest)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Created().json(guest))
}
