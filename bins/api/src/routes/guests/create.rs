use actix_web::{HttpResponse, post, web};
use storage::types::Guest;

use crate::RequestContext;
use crate::rsvp_validation::{valid_email, valid_name};

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
    let name = body.name.trim();
    let email = body.email.trim().to_lowercase();
    if !valid_name(name) || !valid_email(&email) {
        return Ok(HttpResponse::BadRequest().body("Provide a valid name and email address."));
    }
    let guest = Guest::new(name, email, body.plus_one);

    let guest = ctx
        .storage()
        .guests()
        .create(&guest)
        .await
        .map_err(|error| {
            if error
                .as_database_error()
                .is_some_and(|error| error.is_unique_violation())
            {
                actix_web::error::ErrorConflict("A guest already exists with this email.")
            } else {
                eprintln!(
                    "Guest creation storage error [{}]: {error}",
                    ctx.request_id()
                );
                actix_web::error::ErrorInternalServerError("Unable to save guest.")
            }
        })?;

    Ok(HttpResponse::Created().json(guest))
}
