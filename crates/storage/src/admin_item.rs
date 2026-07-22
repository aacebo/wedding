use sqlx::PgPool;

use crate::types::{
    DeadlineRow, EventRow, LlmRun, NewAdminTodo, NewDeadline, NewLlmRun, NewTimelineEvent, TodoRow,
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

    /// Inserts a todo attributed to `created_by` (`ai` for extraction output,
    /// `human` for manual entries). Human items are never touched by re-runs.
    pub async fn insert_todo(
        &self,
        t: &NewAdminTodo,
        created_by: &str,
    ) -> Result<uuid::Uuid, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            INSERT INTO todos_admin
                (source_id, title, owner, due_date, priority, confidence, notes, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
        )
        .bind(t.source_id)
        .bind(&t.title)
        .bind(&t.owner)
        .bind(t.due_date)
        .bind(&t.priority)
        .bind(t.confidence)
        .bind(&t.notes)
        .bind(created_by)
        .fetch_one(self.pool)
        .await
    }

    pub async fn insert_event(
        &self,
        e: &NewTimelineEvent,
        created_by: &str,
    ) -> Result<uuid::Uuid, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            INSERT INTO timeline_events
                (source_id, title, event_date, category, confidence, created_by)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id
            "#,
        )
        .bind(e.source_id)
        .bind(&e.title)
        .bind(e.event_date)
        .bind(&e.category)
        .bind(e.confidence)
        .bind(created_by)
        .fetch_one(self.pool)
        .await
    }

    pub async fn insert_deadline(
        &self,
        d: &NewDeadline,
        created_by: &str,
    ) -> Result<uuid::Uuid, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            INSERT INTO deadlines
                (source_id, title, due_date, severity, confidence, created_by)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id
            "#,
        )
        .bind(d.source_id)
        .bind(&d.title)
        .bind(d.due_date)
        .bind(&d.severity)
        .bind(d.confidence)
        .bind(created_by)
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

    // --- todo / deadline / event mutations -----------------------------------

    /// Updates a todo's editable fields. Flips `created_by` to `human` so the
    /// edit survives future extraction runs (which only replace AI/open items).
    pub async fn update_todo(
        &self,
        id: uuid::Uuid,
        title: &str,
        owner: Option<&str>,
        due_date: Option<chrono::NaiveDate>,
        priority: &str,
        notes: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE todos_admin
               SET title = $2, owner = $3, due_date = $4, priority = $5, notes = $6,
                   created_by = 'human', updated_at = NOW()
             WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(title)
        .bind(owner)
        .bind(due_date)
        .bind(priority)
        .bind(notes)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_todo_status(&self, id: uuid::Uuid, status: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE todos_admin SET status = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(status)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_todo(&self, id: uuid::Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM todos_admin WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_deadline_status(
        &self,
        id: uuid::Uuid,
        status: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE deadlines SET status = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(status)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_deadline(&self, id: uuid::Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM deadlines WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_event(&self, id: uuid::Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM timeline_events WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    // --- reads (joined with source for drill-through) ------------------------

    pub async fn list_todos(&self) -> Result<Vec<TodoRow>, sqlx::Error> {
        sqlx::query_as::<_, TodoRow>(
            r#"
            SELECT t.id, t.source_id, t.title, t.owner, t.due_date, t.priority,
                   t.status, t.confidence, t.notes, t.created_by,
                   c.title AS source_title, c.url AS source_url
              FROM todos_admin t
              LEFT JOIN comm_sources c ON c.id = t.source_id
             ORDER BY
               CASE t.status WHEN 'open' THEN 0 ELSE 1 END,
               t.due_date ASC NULLS LAST,
               t.created_at DESC
            "#,
        )
        .fetch_all(self.pool)
        .await
    }

    pub async fn list_deadlines(&self) -> Result<Vec<DeadlineRow>, sqlx::Error> {
        sqlx::query_as::<_, DeadlineRow>(
            r#"
            SELECT d.id, d.source_id, d.title, d.due_date, d.severity, d.status,
                   d.confidence, d.created_by,
                   c.title AS source_title, c.url AS source_url
              FROM deadlines d
              LEFT JOIN comm_sources c ON c.id = d.source_id
             ORDER BY d.due_date ASC NULLS LAST
            "#,
        )
        .fetch_all(self.pool)
        .await
    }

    pub async fn list_events(&self) -> Result<Vec<EventRow>, sqlx::Error> {
        sqlx::query_as::<_, EventRow>(
            r#"
            SELECT e.id, e.source_id, e.title, e.event_date, e.category,
                   e.confidence, e.created_by,
                   c.title AS source_title, c.url AS source_url
              FROM timeline_events e
              LEFT JOIN comm_sources c ON c.id = e.source_id
             ORDER BY e.event_date ASC NULLS LAST
            "#,
        )
        .fetch_all(self.pool)
        .await
    }

    /// Open, dated deadlines from today onward — the dashboard's "what's next".
    pub async fn upcoming_deadlines(&self, limit: i64) -> Result<Vec<DeadlineRow>, sqlx::Error> {
        sqlx::query_as::<_, DeadlineRow>(
            r#"
            SELECT d.id, d.source_id, d.title, d.due_date, d.severity, d.status,
                   d.confidence, d.created_by,
                   c.title AS source_title, c.url AS source_url
              FROM deadlines d
              LEFT JOIN comm_sources c ON c.id = d.source_id
             WHERE d.status = 'open' AND d.due_date IS NOT NULL AND d.due_date >= CURRENT_DATE
             ORDER BY d.due_date ASC
             LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(self.pool)
        .await
    }

    /// Count of still-open todos, for the dashboard summary.
    pub async fn open_todo_count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM todos_admin WHERE status = 'open'")
            .fetch_one(self.pool)
            .await
    }

    /// Aggregate totals for the extract/status responses (todos, events, deadlines).
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
