use actix_session::Session;
use actix_web::{HttpResponse, post};

/// Clears the admin session cookie.
#[post("/admin/logout")]
pub async fn post(session: Session) -> HttpResponse {
    session.purge();
    HttpResponse::NoContent().finish()
}
