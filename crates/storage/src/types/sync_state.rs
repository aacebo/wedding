/// Per-account, per-provider incremental sync cursor. For Gmail the cursor is
/// the epoch-seconds high-water mark used in the next `after:` query; for Drive
/// it is the changes-feed page token.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct SyncState {
    pub account_email: String,
    pub provider: String,
    pub cursor: Option<String>,
    pub last_synced_at: Option<chrono::DateTime<chrono::Utc>>,
}
