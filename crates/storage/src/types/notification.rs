use chrono::{DateTime, NaiveDate, Utc};

/// An in-page reminder derived from a deadline or a due todo. `kind` is one of
/// `deadline_soon`, `deadline_overdue`, or `todo_due`; `status` is `unread`,
/// `read`, or `dismissed`. Reminders are regenerated idempotently — one row per
/// referenced item (`ref_type` + `ref_id`) whose `kind` escalates in place.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct Notification {
    pub id: uuid::Uuid,
    pub kind: String,
    pub ref_type: String,
    pub ref_id: uuid::Uuid,
    pub title: String,
    pub body: String,
    pub due_date: Option<NaiveDate>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Notification {
    pub fn is_overdue(&self) -> bool {
        self.kind == "deadline_overdue"
    }

    pub fn is_unread(&self) -> bool {
        self.status == "unread"
    }
}
