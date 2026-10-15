use actix_web::{HttpResponse, post, web};
use storage::types::Rsvp;

use crate::RequestContext;
use crate::rsvp_validation::valid_party_size;

#[derive(Debug, serde::Deserialize)]
struct CreateRsvp {
    pub attending: bool,
    pub party_size: Option<i16>,
    pub meal_preference: Option<String>,
}

#[post("/guests/{guest_id}/rsvps")]
pub async fn post(
    ctx: RequestContext,
    path: web::Path<uuid::Uuid>,
    body: web::Json<CreateRsvp>,
) -> Result<HttpResponse, actix_web::Error> {
    let guest_id = path.into_inner();
    let party_size = body.party_size.unwrap_or(i16::from(body.attending));
    if !valid_party_size(body.attending, party_size) {
        return Ok(HttpResponse::BadRequest()
            .body("Attending parties must contain 1 or 2 people; declines must contain 0."));
    }
    if ctx
        .storage()
        .guests()
        .get(guest_id)
        .await
        .map_err(|error| {
            eprintln!(
                "RSVP guest lookup storage error [{}]: {error}",
                ctx.request_id()
            );
            actix_web::error::ErrorInternalServerError("Unable to find guest.")
        })?
        .is_none()
    {
        return Ok(HttpResponse::NotFound().body("Guest not found."));
    }
    let rsvp = Rsvp::new(
        guest_id,
        body.attending,
        party_size,
        body.meal_preference.clone(),
    );

    let rsvp = ctx.storage().rsvps().create(&rsvp).await.map_err(|error| {
        if error
            .as_database_error()
            .is_some_and(|error| error.is_unique_violation())
        {
            actix_web::error::ErrorConflict("An RSVP already exists for this guest.")
        } else {
            eprintln!("RSVP API storage error [{}]: {error}", ctx.request_id());
            actix_web::error::ErrorInternalServerError("Unable to save RSVP.")
        }
    })?;

    Ok(HttpResponse::Created().json(rsvp))
}
