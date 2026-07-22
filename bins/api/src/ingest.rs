use chrono::{DateTime, TimeZone, Utc};

use google::{DriveClient, GmailClient};
use storage::Storage;
use storage::types::NewCommSource;
use storage::UpsertOutcome;

use crate::google_auth::{GoogleAuth, GoogleAuthError};

const GMAIL_PROVIDER: &str = "gmail";
const DRIVE_PROVIDER: &str = "drive";

/// Only look back a year on the very first Gmail sync; later syncs are bounded
/// by the stored `after:` cursor instead.
const GMAIL_BASE_QUERY: &str = "newer_than:1y";
/// Google Docs only (native docs export cleanly to text); trashed excluded.
const DRIVE_SEED_QUERY: &str =
    "trashed = false and mimeType = 'application/vnd.google-apps.document'";
/// Safety cap on items fetched per provider per sync run.
const MAX_ITEMS: usize = 500;
const SNIPPET_LEN: usize = 240;

#[derive(thiserror::Error, Debug)]
enum IngestError {
    #[error(transparent)]
    Google(#[from] google::GoogleError),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Auth(#[from] GoogleAuthError),
}

#[derive(serde::Serialize, Default)]
pub struct ProviderResult {
    pub new: u64,
    pub updated: u64,
    pub skipped: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(serde::Serialize)]
pub struct AccountReport {
    pub account_email: String,
    pub gmail: ProviderResult,
    pub drive: ProviderResult,
}

#[derive(serde::Serialize)]
pub struct SyncReport {
    pub accounts: Vec<AccountReport>,
    pub total_sources: i64,
}

/// Runs Gmail + Drive ingestion for every linked account. Failures are isolated
/// per account and per provider so one broken token can't abort the whole run.
pub async fn sync_all(storage: &Storage<'_>, google: &GoogleAuth) -> Result<SyncReport, sqlx::Error> {
    let accounts = storage.google().list().await?;
    let mut reports = Vec::with_capacity(accounts.len());

    for account in accounts {
        let email = account.email;

        // A single valid token drives both providers; if it can't be obtained
        // (e.g. revoked refresh token) both providers report the same error.
        let token = match google.valid_access_token(storage, &email).await {
            Ok(t) => t,
            Err(e) => {
                let msg = e.to_string();
                reports.push(AccountReport {
                    account_email: email,
                    gmail: ProviderResult { error: Some(msg.clone()), ..Default::default() },
                    drive: ProviderResult { error: Some(msg), ..Default::default() },
                });
                continue;
            }
        };

        let gmail = to_result(sync_gmail(storage, &email, &token).await);
        let drive = to_result(sync_drive(storage, &email, &token).await);

        reports.push(AccountReport {
            account_email: email,
            gmail,
            drive,
        });
    }

    let total_sources = storage.comm_sources().count().await?;
    Ok(SyncReport {
        accounts: reports,
        total_sources,
    })
}

fn to_result(r: Result<ProviderResult, IngestError>) -> ProviderResult {
    r.unwrap_or_else(|e| ProviderResult {
        error: Some(e.to_string()),
        ..Default::default()
    })
}

async fn sync_gmail(
    storage: &Storage<'_>,
    email: &str,
    access_token: &str,
) -> Result<ProviderResult, IngestError> {
    let cursor = storage
        .sync_state()
        .get(email, GMAIL_PROVIDER)
        .await?
        .and_then(|s| s.cursor);

    // Incremental via an epoch-seconds high-water mark passed to Gmail's
    // `after:` operator; the UNIQUE(provider, external_id) constraint absorbs
    // any overlap at the boundary second.
    let query = match cursor.as_deref() {
        Some(secs) => format!("{GMAIL_BASE_QUERY} after:{secs}"),
        None => GMAIL_BASE_QUERY.to_string(),
    };

    let client = GmailClient::new(access_token);
    let ids = client.list_message_ids(Some(&query), MAX_ITEMS).await?;

    let mut result = ProviderResult::default();
    let mut max_ms: Option<i64> = None;

    for id in ids {
        let msg = client.get_message(&id).await?;
        if let Some(ms) = msg.internal_date_ms {
            max_ms = Some(max_ms.map_or(ms, |m| m.max(ms)));
        }

        let source = NewCommSource {
            account_email: email.to_string(),
            provider: GMAIL_PROVIDER.to_string(),
            external_id: msg.id.clone(),
            kind: "email".to_string(),
            sender: msg.sender,
            title: msg.subject.unwrap_or_else(|| "(no subject)".to_string()),
            snippet: truncate(&msg.snippet),
            body_text: msg.body_text,
            url: Some(format!("https://mail.google.com/mail/u/0/#all/{}", msg.thread_id)),
            occurred_at: msg.internal_date_ms.and_then(ms_to_dt),
            raw: msg.raw,
        };
        record(&mut result, storage.comm_sources().upsert(&source).await?);
    }

    // Advance the cursor to one second past the newest message so the next run
    // doesn't re-list everything we just saw.
    let new_cursor = match max_ms {
        Some(ms) => Some((ms / 1000 + 1).to_string()),
        None => cursor,
    };
    storage
        .sync_state()
        .upsert(email, GMAIL_PROVIDER, new_cursor.as_deref())
        .await?;

    Ok(result)
}

async fn sync_drive(
    storage: &Storage<'_>,
    email: &str,
    access_token: &str,
) -> Result<ProviderResult, IngestError> {
    let cursor = storage
        .sync_state()
        .get(email, DRIVE_PROVIDER)
        .await?
        .and_then(|s| s.cursor);

    let client = DriveClient::new(access_token);
    let mut result = ProviderResult::default();

    let new_cursor = match cursor {
        // Incremental: pull only files that changed since the stored page token.
        Some(token) => {
            let changes = client.list_changes(&token).await?;
            for file in &changes.files {
                if file.removed {
                    storage
                        .comm_sources()
                        .mark_removed(DRIVE_PROVIDER, &file.id)
                        .await?;
                    result.skipped += 1;
                    continue;
                }
                ingest_drive_file(storage, email, &client, file, &mut result).await?;
            }
            changes.new_start_page_token.unwrap_or(token)
        }
        // First sync: seed from a files.list, then record the start token so the
        // next run switches to the incremental changes feed.
        None => {
            let files = client.list_files(Some(DRIVE_SEED_QUERY), MAX_ITEMS).await?;
            for file in &files {
                ingest_drive_file(storage, email, &client, file, &mut result).await?;
            }
            client.start_page_token().await?
        }
    };

    storage
        .sync_state()
        .upsert(email, DRIVE_PROVIDER, Some(&new_cursor))
        .await?;

    Ok(result)
}

async fn ingest_drive_file(
    storage: &Storage<'_>,
    email: &str,
    client: &DriveClient,
    file: &google::DriveFile,
    result: &mut ProviderResult,
) -> Result<(), IngestError> {
    let body = client.export_text(file).await?;
    let occurred_at = file
        .modified_time
        .as_deref()
        .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let source = NewCommSource {
        account_email: email.to_string(),
        provider: DRIVE_PROVIDER.to_string(),
        external_id: file.id.clone(),
        kind: "doc".to_string(),
        sender: None,
        title: file.name.clone(),
        snippet: truncate(&body),
        body_text: body,
        url: file.web_view_link.clone(),
        occurred_at,
        raw: file.raw.clone(),
    };
    record(result, storage.comm_sources().upsert(&source).await?);
    Ok(())
}

fn record(result: &mut ProviderResult, outcome: UpsertOutcome) {
    match outcome {
        UpsertOutcome::Inserted => result.new += 1,
        UpsertOutcome::Updated => result.updated += 1,
    }
}

fn ms_to_dt(ms: i64) -> Option<DateTime<Utc>> {
    Utc.timestamp_millis_opt(ms).single()
}

fn truncate(s: &str) -> String {
    let cleaned = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.chars().count() <= SNIPPET_LEN {
        cleaned
    } else {
        let mut out: String = cleaned.chars().take(SNIPPET_LEN).collect();
        out.push('…');
        out
    }
}
