use base64::Engine;
use serde::Deserialize;

use crate::error::GoogleError;

const GMAIL_BASE: &str = "https://gmail.googleapis.com/gmail/v1/users/me";

/// A normalized Gmail message ready to be persisted as a communication source.
/// Dates are surfaced as `internal_date_ms` (epoch millis) so this crate stays
/// free of a datetime dependency — the caller converts.
pub struct GmailMessage {
    pub id: String,
    pub thread_id: String,
    pub sender: Option<String>,
    pub subject: Option<String>,
    pub snippet: String,
    pub body_text: String,
    pub internal_date_ms: Option<i64>,
    pub raw: serde_json::Value,
}

/// Read-only Gmail API client bound to a single access token.
pub struct GmailClient {
    http: reqwest::Client,
    access_token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListResponse {
    #[serde(default)]
    messages: Vec<MessageRef>,
    #[serde(default)]
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
struct MessageRef {
    id: String,
}

impl GmailClient {
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            access_token: access_token.into(),
        }
    }

    /// Lists message ids matching `query` (Gmail search syntax, e.g.
    /// `after:1700000000`), following pagination up to `max` ids.
    pub async fn list_message_ids(
        &self,
        query: Option<&str>,
        max: usize,
    ) -> Result<Vec<String>, GoogleError> {
        let mut ids = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut req = self
                .http
                .get(format!("{GMAIL_BASE}/messages"))
                .bearer_auth(&self.access_token)
                .query(&[("maxResults", "100")]);
            if let Some(q) = query {
                req = req.query(&[("q", q)]);
            }
            if let Some(ref t) = page_token {
                req = req.query(&[("pageToken", t.as_str())]);
            }

            let res = req.send().await?;
            let body: ListResponse = parse_json(res).await?;
            ids.extend(body.messages.into_iter().map(|m| m.id));

            if ids.len() >= max {
                ids.truncate(max);
                break;
            }
            match body.next_page_token {
                Some(t) => page_token = Some(t),
                None => break,
            }
        }

        Ok(ids)
    }

    /// Fetches a single message (format=full) and normalizes it.
    pub async fn get_message(&self, id: &str) -> Result<GmailMessage, GoogleError> {
        let res = self
            .http
            .get(format!("{GMAIL_BASE}/messages/{id}"))
            .bearer_auth(&self.access_token)
            .query(&[("format", "full")])
            .send()
            .await?;
        let raw: serde_json::Value = parse_json(res).await?;

        let snippet = raw
            .get("snippet")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let thread_id = raw
            .get("threadId")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let internal_date_ms = raw
            .get("internalDate")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<i64>().ok());

        let payload = raw.get("payload");
        let sender = payload.and_then(|p| header_value(p, "From"));
        let subject = payload.and_then(|p| header_value(p, "Subject"));
        let body_text = payload
            .map(extract_body_text)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| snippet.clone());

        Ok(GmailMessage {
            id: id.to_string(),
            thread_id,
            sender,
            subject,
            snippet,
            body_text,
            internal_date_ms,
            raw,
        })
    }
}

/// Reads a header value (case-insensitive) from a message payload.
fn header_value(payload: &serde_json::Value, name: &str) -> Option<String> {
    payload
        .get("headers")?
        .as_array()?
        .iter()
        .find(|h| {
            h.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n.eq_ignore_ascii_case(name))
                .unwrap_or(false)
        })
        .and_then(|h| h.get("value"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Recursively walks a MIME payload collecting decoded `text/plain` content,
/// falling back to `text/html` when no plain text part exists.
fn extract_body_text(payload: &serde_json::Value) -> String {
    let mut plain = String::new();
    let mut html = String::new();
    collect_parts(payload, &mut plain, &mut html);
    if !plain.trim().is_empty() {
        plain
    } else {
        strip_html(&html)
    }
}

fn collect_parts(part: &serde_json::Value, plain: &mut String, html: &mut String) {
    let mime = part.get("mimeType").and_then(|v| v.as_str()).unwrap_or("");

    if let Some(data) = part
        .get("body")
        .and_then(|b| b.get("data"))
        .and_then(|d| d.as_str())
        && let Some(decoded) = decode_b64url(data)
    {
        if mime == "text/plain" {
            plain.push_str(&decoded);
            plain.push('\n');
        } else if mime == "text/html" {
            html.push_str(&decoded);
            html.push('\n');
        }
    }

    if let Some(parts) = part.get("parts").and_then(|p| p.as_array()) {
        for child in parts {
            collect_parts(child, plain, html);
        }
    }
}

fn decode_b64url(data: &str) -> Option<String> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(data.trim())
        .ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// Extremely small HTML-to-text fallback: drops tags and collapses whitespace.
fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

async fn parse_json<T: serde::de::DeserializeOwned>(
    res: reqwest::Response,
) -> Result<T, GoogleError> {
    if !res.status().is_success() {
        let status = res.status().as_u16();
        return Err(GoogleError::Api(status, res.text().await.unwrap_or_default()));
    }
    Ok(res.json::<T>().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_html_collapses() {
        let out = strip_html("<p>Hello   <b>world</b></p>");
        assert_eq!(out, "Hello world");
    }

    #[test]
    fn extract_prefers_plain_text() {
        let payload = serde_json::json!({
            "mimeType": "multipart/alternative",
            "parts": [
                {"mimeType": "text/plain", "body": {"data": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("plain body")}},
                {"mimeType": "text/html", "body": {"data": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("<b>html body</b>")}}
            ]
        });
        assert_eq!(extract_body_text(&payload).trim(), "plain body");
    }

    #[test]
    fn header_lookup_is_case_insensitive() {
        let payload = serde_json::json!({
            "headers": [{"name": "Subject", "value": "Catering"}]
        });
        assert_eq!(header_value(&payload, "subject").as_deref(), Some("Catering"));
    }
}
