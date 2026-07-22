use chrono::{DateTime, Utc};

/// Audit record of one background/manual pipeline pass (ingest → extract →
/// notify). `trigger` is `manual` or `scheduled`; `status` is `running`,
/// `success`, `error`, or `skipped` (another run held the advisory lock).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct SyncRun {
    pub id: uuid::Uuid,
    pub trigger: String,
    pub status: String,
    pub sources_new: i32,
    pub sources_updated: i32,
    pub todos_created: i32,
    pub events_created: i32,
    pub deadlines_created: i32,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl SyncRun {
    pub fn ok(&self) -> bool {
        self.status == "success"
    }
}

/// Counts rolled up over a single pipeline pass, persisted on completion.
#[derive(Debug, Clone, Default)]
pub struct SyncRunCounts {
    pub sources_new: i32,
    pub sources_updated: i32,
    pub todos_created: i32,
    pub events_created: i32,
    pub deadlines_created: i32,
}
