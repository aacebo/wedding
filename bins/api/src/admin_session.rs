use std::future::{Ready, ready};

use actix_session::SessionExt;
use actix_web::{Error, FromRequest, HttpRequest, error::ErrorNotFound, web};

use admin::SESSION_EMAIL_KEY;

use crate::Context;

/// Extractor that proves the caller is a signed-in, allowlisted admin.
///
/// Any handler that takes an `AdminSession` argument is automatically gated:
/// unauthenticated or non-allowlisted callers get a **404 Not Found** (rather
/// than a 401) so the very existence of `/admin` stays hidden.
pub struct AdminSession {
    pub email: String,
}

impl FromRequest for AdminSession {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let email = req
            .get_session()
            .get::<String>(SESSION_EMAIL_KEY)
            .ok()
            .flatten();

        let allowed = matches!(
            (&email, req.app_data::<web::Data<Context>>()),
            (Some(email), Some(ctx)) if ctx.allowlist().contains(email)
        );

        match email {
            Some(email) if allowed => ready(Ok(AdminSession { email })),
            // Deliberately 404, not 401/403 — keep the page hidden.
            _ => ready(Err(ErrorNotFound("Not Found"))),
        }
    }
}
