use std::time::Duration;

use actix_web::rt;
use serde::Serialize;
use storage::PendingRsvpSheetSync;

use crate::Context;

#[derive(Serialize)]
struct WebhookRequest<'a> {
    secret: &'a str,
    rsvp: &'a serde_json::Value,
}

#[derive(serde::Deserialize)]
struct WebhookResponse {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
}

pub fn spawn(
    ctx: Context,
    webhook_url: Option<String>,
    webhook_secret: Option<String>,
) {
    let (Some(webhook_url), Some(webhook_secret)) = (webhook_url, webhook_secret) else {
        eprintln!("Google Sheets RSVP sync is not configured; submissions remain safely queued in PostgreSQL.");
        return;
    };

    rt::spawn(async move {
        let client = reqwest::Client::new();
        loop {
            let pending = match ctx.storage().rsvps().pending_sheet_syncs(25).await {
                Ok(pending) => pending,
                Err(error) => {
                    eprintln!("Google Sheets outbox read failed: {error}");
                    rt::time::sleep(Duration::from_secs(30)).await;
                    continue;
                }
            };

            for item in pending {
                if let Err(error) = send(&client, &webhook_url, &webhook_secret, &item).await {
                    eprintln!("Google Sheets sync failed for RSVP {}: {error}", item.rsvp_id);
                    if let Err(mark_error) = ctx
                        .storage()
                        .rsvps()
                        .mark_sheet_sync_failed(item.rsvp_id, &error)
                        .await
                    {
                        eprintln!("Could not record Google Sheets sync failure: {mark_error}");
                    }
                } else if let Err(error) = ctx
                    .storage()
                    .rsvps()
                    .mark_sheet_sync_delivered(item.rsvp_id)
                    .await
                {
                    eprintln!(
                        "Google Sheets sync succeeded but outbox acknowledgement failed for RSVP {}: {error}",
                        item.rsvp_id
                    );
                }
            }

            rt::time::sleep(Duration::from_secs(30)).await;
        }
    });
}

async fn send(
    client: &reqwest::Client,
    webhook_url: &str,
    webhook_secret: &str,
    item: &PendingRsvpSheetSync,
) -> Result<(), String> {
    let response = client
        .post(webhook_url)
        .json(&WebhookRequest {
            secret: webhook_secret,
            rsvp: &item.payload,
        })
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;

    let body = response
        .json::<WebhookResponse>()
        .await
        .map_err(|error| format!("invalid webhook response: {error}"))?;
    if body.ok {
        Ok(())
    } else {
        Err(body.error.unwrap_or_else(|| "webhook rejected the RSVP".to_owned()))
    }
}
