use crate::notion::{GuestError, Notion};
use actix_web::{HttpRequest, HttpResponse, error::ErrorServiceUnavailable, post, web};
use serde_json::Value;

/// Receives either a Notion automation or a Notion connection webhook.
#[post("/webhooks/notion/rsvp-progress")]
pub async fn refresh(
    request: HttpRequest,
    body: web::Bytes,
    notion: web::Data<Notion>,
    secret: web::Data<Option<String>>,
) -> Result<HttpResponse, actix_web::Error> {
    let header_secret = request
        .headers()
        .get("X-Notion-Progress-Secret")
        .and_then(|value| value.to_str().ok());
    let query_secret = request.query_string().split('&').find_map(|part| {
        part.split_once('=')
            .filter(|(key, _)| *key == "token")
            .map(|(_, value)| value)
    });
    if secret.as_deref().is_none()
        || (secret.as_deref() != header_secret && secret.as_deref() != query_secret)
    {
        return Ok(HttpResponse::Unauthorized().finish());
    }
    if let Ok(Value::Object(payload)) = serde_json::from_slice::<Value>(&body) {
        if let Some(token) = payload.get("verification_token").and_then(Value::as_str) {
            eprintln!("Notion webhook verification token: {token}");
        }
    }
    notion
        .refresh_progress()
        .await
        .map_err(|error| match error {
            GuestError::InvalidCode | GuestError::Unavailable => {
                ErrorServiceUnavailable("Unable to refresh RSVP progress")
            }
        })?;
    Ok(HttpResponse::NoContent().finish())
}
