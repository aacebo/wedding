use actix_session::Session;
use actix_web::{HttpResponse, error::ErrorNotFound, get, web};

use admin::OAUTH_STATE_KEY;

use crate::Context;

/// Generates an unguessable `state` value for CSRF protection on the OAuth
/// round-trip (two random v4 UUIDs → 256 bits of entropy).
fn generate_state() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

/// Begins Google SSO: stores a fresh `state` in the session and redirects the
/// user to Google's consent screen. 404s (keeping `/admin` hidden) when Google
/// SSO is not configured.
#[get("/admin/auth/google/login")]
pub async fn get(
    ctx: web::Data<Context>,
    session: Session,
) -> Result<HttpResponse, actix_web::Error> {
    let google = ctx.google().ok_or_else(|| ErrorNotFound("Not Found"))?;

    let state = generate_state();
    session
        .insert(OAUTH_STATE_KEY, &state)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let url = google.authorize_url(&state);
    Ok(HttpResponse::Found()
        .append_header(("Location", url))
        .finish())
}
