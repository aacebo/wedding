use actix_session::Session;
use actix_web::{HttpResponse, error::ErrorNotFound, post, web};

use admin::SESSION_EMAIL_KEY;

use crate::Context;

#[derive(serde::Deserialize)]
struct DevLogin {
    email: String,
}

/// Temporary development-only login used to establish an admin session before
/// real Google SSO (Phase 02) exists. Disabled unless `ADMIN_DEV_LOGIN=true`,
/// and only accepts allowlisted emails. When disabled it 404s like any other
/// non-existent route, keeping the surface hidden.
#[post("/admin/dev-login")]
pub async fn post(
    ctx: web::Data<Context>,
    session: Session,
    body: web::Json<DevLogin>,
) -> Result<HttpResponse, actix_web::Error> {
    if !ctx.dev_login_enabled() {
        return Err(ErrorNotFound("Not Found"));
    }

    let email = body.email.trim().to_lowercase();
    if !ctx.allowlist().contains(&email) {
        return Err(ErrorNotFound("Not Found"));
    }

    session
        .insert(SESSION_EMAIL_KEY, &email)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::NoContent().finish())
}
