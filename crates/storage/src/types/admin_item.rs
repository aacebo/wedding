use chrono::{DateTime, NaiveDate, Utc};

/// A task surfaced from a source (or hand-added). `created_by` distinguishes AI
/// output from human entries; the extraction pipeline only ever deletes/replaces
/// AI-created, still-`open` rows so it never clobbers human edits.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct AdminTodo {
    pub id: uuid::Uuid,
    pub source_id: Option<uuid::Uuid>,
    pub title: String,
    pub owner: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub priority: String,
    pub status: String,
    pub confidence: f32,
    pub notes: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct TimelineEvent {
    pub id: uuid::Uuid,
    pub source_id: Option<uuid::Uuid>,
    pub title: String,
    pub event_date: Option<NaiveDate>,
    pub category: Option<String>,
    pub confidence: f32,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct Deadline {
    pub id: uuid::Uuid,
    pub source_id: Option<uuid::Uuid>,
    pub title: String,
    pub due_date: Option<NaiveDate>,
    pub severity: String,
    pub status: String,
    pub confidence: f32,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Audit record of a single extraction call (one per processed source).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct LlmRun {
    pub id: uuid::Uuid,
    pub source_id: Option<uuid::Uuid>,
    pub model: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub cost_estimate: f32,
    pub status: String,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Fields supplied when inserting an AI-extracted todo.
#[derive(Debug, Clone)]
pub struct NewAdminTodo {
    pub source_id: Option<uuid::Uuid>,
    pub title: String,
    pub owner: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub priority: String,
    pub confidence: f32,
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewTimelineEvent {
    pub source_id: Option<uuid::Uuid>,
    pub title: String,
    pub event_date: Option<NaiveDate>,
    pub category: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct NewDeadline {
    pub source_id: Option<uuid::Uuid>,
    pub title: String,
    pub due_date: Option<NaiveDate>,
    pub severity: String,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct NewLlmRun {
    pub source_id: Option<uuid::Uuid>,
    pub model: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub cost_estimate: f32,
    pub status: String,
    pub error: Option<String>,
}
