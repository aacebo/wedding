use sqlx::PgPool;

use crate::types::{
    AdminTodo, Deadline, LlmRun, NewAdminTodo, NewDeadline, NewLlmRun, NewTimelineEvent,
    TimelineEvent,
};

/// Storage for AI/human-curated planning items (todos, timeline events,
/// deadlines) and the extraction audit log.
pub struct AdminItemStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> AdminItemStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    // --- idempotency helpers -------------------------------------------------

    /// Clears AI-created, still-`open` items for a source before re-inserting
    /// fresh extraction output. Human entries and any item whose status changed
    /// (done/dismissed) are left untouched, so re-runs never clobber edits.
    pub async fn clear_ai_open_for_source(&self, source_id: uuid::Uuid) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        for table in ["todos_admin", "deadlines"] {
            sqlx::query(&format!(
                "DELETE FROM {table} WHERE source_id = $1 AND created_by = 'ai' AND status = 'open'"
            ))
            .bind(source_id)
            .execute(&mut *tx)
            .await?;
        }
        // timeline_events has no status column; replace all AI-created rows.
        sqlx::query("DELETE FROM timeline_events WHERE source_id = $1 AND created_by = 'ai'")
            .bind(source_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    // --- inserts -------------------------------------------------------------

    pub async fn insert_todo(&self, t: &NewAdminTodo) -> Result<AdminTodo, sqlx::Error> {
        sqlx::query_as::<_, AdminTodo>(
            r#"
            INSERT INTO todos_admin
                (source_id, title, owner, due_date, priority, confidence, notes, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'ai')
            RETURNING *
            "#,
        )
        .bind(t.source_id)
        .bind(&t.title)
        .bind(&t.owner)
        .bind(t.due_date)
        .bind(&t.priority)
        .bind(t.confidence)
        .bind(&t.notes)
        .fetch_one(self.pool)
        .await
    }

    pub async fn insert_event(&self, e: &NewTimelineEvent) -> Result<TimelineEvent, sqlx::Error> {
        sqlx::query_as::<_, TimelineEvent>(
            r#"
            INSERT INTO timeline_events
                (source_id, title, event_date, category, confidence, created_by)
            VALUES ($1, $2, $3, $4, $5, 'ai')
            RETURNING *
            "#,
        )
        .bind(e.source_id)
        .bind(&e.title)
        .bind(e.event_date)
        .bind(&e.category)
        .bind(e.confidence)
        .fetch_one(self.pool)
        .await
    }

    pub async fn insert_deadline(&self, d: &NewDeadline) -> Result<Deadline, sqlx::Error> {
        sqlx::query_as::<_, Deadline>(
            r#"
            INSERT INTO deadlines
                (source_id, title, due_date, severity, confidence, created_by)
            VALUES ($1, $2, $3, $4, $5, 'ai')
            RETURNING *
            "#,
        )
        .bind(d.source_id)
        .bind(&d.title)
        .bind(d.due_date)
        .bind(&d.severity)
        .bind(d.confidence)
        .fetch_one(self.pool)
        .await
    }

    pub async fn insert_llm_run(&self, r: &NewLlmRun) -> Result<LlmRun, sqlx::Error> {
        sqlx::query_as::<_, LlmRun>(
            r#"
            INSERT INTO llm_runs
                (source_id, model, prompt_tokens, completion_tokens, cost_estimate, status, error)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(r.source_id)
        .bind(&r.model)
        .bind(r.prompt_tokens)
        .bind(r.completion_tokens)
        .bind(r.cost_estimate)
        .bind(&r.status)
        .bind(&r.error)
        .fetch_one(self.pool)
        .await
    }

    // --- reads (used by later phases / status) -------------------------------

    pub async fn list_todos(&self) -> Result<Vec<AdminTodo>, sqlx::Error> {
        sqlx::query_as::<_, AdminTodo>(
            "SELECT * FROM todos_admin ORDER BY due_date ASC NULLS LAST, created_at DESC",
        )
        .fetch_all(self.pool)
        .await
    }

    pub async fn list_events(&self) -> Result<Vec<TimelineEvent>, sqlx::Error> {
        sqlx::query_as::<_, TimelineEvent>(
            "SELECT * FROM timeline_events ORDER BY event_date ASC NULLS LAST",
        )
        .fetch_all(self.pool)
        .await
    }

    pub async fn list_deadlines(&self) -> Result<Vec<Deadline>, sqlx::Error> {
        sqlx::query_as::<_, Deadline>(
            "SELECT * FROM deadlines ORDER BY due_date ASC NULLS LAST",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Aggregate counts for the extract/status responses.
    pub async fn counts(&self) -> Result<(i64, i64, i64), sqlx::Error> {
        let todos: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM todos_admin")
            .fetch_one(self.pool)
            .await?;
        let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM timeline_events")
            .fetch_one(self.pool)
            .await?;
        let deadlines: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deadlines")
            .fetch_one(self.pool)
            .await?;
        Ok((todos, events, deadlines))
    }
}
