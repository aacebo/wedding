use actix_web::{HttpResponse, post, web};
use storage::types::Rsvp;

use crate::RequestContext;

#[derive(Debug, serde::Deserialize)]
struct CreateRsvp {
    pub attending: bool,
    pub meal_preference: Option<String>,
}

#[post("/guests/{guest_id}/rsvps")]
pub async fn post(
    ctx: RequestContext,
    path: web::Path<uuid::Uuid>,
    body: web::Json<CreateRsvp>,
) -> Result<HttpResponse, actix_web::Error> {
    let guest_id = path.into_inner();
    let rsvp = Rsvp::new(guest_id, body.attending, body.meal_preference.clone());

    let rsvp = ctx
        .storage()
        .rsvps()
        .create(&rsvp)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Created().json(rsvp))
}
