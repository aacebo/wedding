use actix_session::Session;
use actix_web::{HttpResponse, error::ErrorNotFound, get, web};

use admin::{OAUTH_STATE_KEY, SESSION_EMAIL_KEY};

use crate::Context;

#[derive(serde::Deserialize)]
pub struct Callback {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// Completes Google SSO. Verifies the `state` against the session, exchanges the
/// code for tokens, fetches the user's identity, and only then enforces the
/// allowlist. Non-allowlisted users get their session purged and a 404 so the
/// admin area stays hidden. On success the email is stored in the session and
/// the encrypted tokens are persisted.
#[get("/admin/auth/google/callback")]
pub async fn get(
    ctx: web::Data<Context>,
    session: Session,
    query: web::Query<Callback>,
) -> Result<HttpResponse, actix_web::Error> {
    let google = ctx.google().ok_or_else(|| ErrorNotFound("Not Found"))?;

    if query.error.is_some() {
        return Err(ErrorNotFound("Not Found"));
    }

    let code = query
        .code
        .as_deref()
        .ok_or_else(|| ErrorNotFound("Not Found"))?;
    let state = query
        .state
        .as_deref()
        .ok_or_else(|| ErrorNotFound("Not Found"))?;

    // CSRF: the state we redirected with must match what comes back.
    let expected: Option<String> = session.get(OAUTH_STATE_KEY).unwrap_or(None);
    match expected {
        Some(ref e) if e == state => {}
        _ => return Err(ErrorNotFound("Not Found")),
    }
    session.remove(OAUTH_STATE_KEY);

    let (info, tokens) = google
        .exchange(code)
        .await
        .map_err(|_| ErrorNotFound("Not Found"))?;

    let email = info.email.trim().to_lowercase();
    if !ctx.allowlist().contains(&email) {
        // Never persist tokens for a non-allowlisted account.
        session.purge();
        return Err(ErrorNotFound("Not Found"));
    }

    google
        .persist(&ctx.storage(), &info, &tokens)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    session
        .insert(SESSION_EMAIL_KEY, &email)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Found()
        .append_header(("Location", "/admin"))
        .finish())
}
