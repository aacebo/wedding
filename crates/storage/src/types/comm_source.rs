/// A normalized communication pulled from Gmail or Drive, deduplicated on
/// `(provider, external_id)`. `processed_at` is set once Phase 04 has turned the
/// source into todos/timeline items.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct CommSource {
    pub id: uuid::Uuid,
    pub account_email: String,
    pub provider: String,
    pub external_id: String,
    pub kind: String,
    pub sender: Option<String>,
    pub title: String,
    pub snippet: String,
    pub body_text: String,
    pub url: Option<String>,
    pub occurred_at: Option<chrono::DateTime<chrono::Utc>>,
    pub raw: serde_json::Value,
    pub processed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// The fields the ingestion layer supplies when upserting a source. `id` and
/// timestamps are assigned by the database.
#[derive(Debug, Clone)]
pub struct NewCommSource {
    pub account_email: String,
    pub provider: String,
    pub external_id: String,
    pub kind: String,
    pub sender: Option<String>,
    pub title: String,
    pub snippet: String,
    pub body_text: String,
    pub url: Option<String>,
    pub occurred_at: Option<chrono::DateTime<chrono::Utc>>,
    pub raw: serde_json::Value,
}
