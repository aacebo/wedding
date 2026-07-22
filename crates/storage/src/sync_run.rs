use sqlx::PgPool;

use crate::types::{SyncRun, SyncRunCounts};

/// Storage for the `sync_runs` audit log and the advisory lock that keeps two
/// pipeline passes (e.g. the cron job and a manual trigger) from overlapping.
pub struct SyncRunStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> SyncRunStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Records the start of a run, returning its id.
    pub async fn start(&self, trigger: &str) -> Result<uuid::Uuid, sqlx::Error> {
        sqlx::query_scalar(
            "INSERT INTO sync_runs (trigger, status) VALUES ($1, 'running') RETURNING id",
        )
        .bind(trigger)
        .fetch_one(self.pool)
        .await
    }

    /// Marks a run finished with its terminal status, counts, and optional error.
    pub async fn finish(
        &self,
        id: uuid::Uuid,
        status: &str,
        counts: &SyncRunCounts,
        error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE sync_runs
               SET status = $2,
                   sources_new = $3, sources_updated = $4,
                   todos_created = $5, events_created = $6, deadlines_created = $7,
                   error = $8, finished_at = NOW()
             WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(counts.sources_new)
        .bind(counts.sources_updated)
        .bind(counts.todos_created)
        .bind(counts.events_created)
        .bind(counts.deadlines_created)
        .bind(error)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// The most recent run, for the dashboard "system status" widget.
    pub async fn latest(&self) -> Result<Option<SyncRun>, sqlx::Error> {
        sqlx::query_as::<_, SyncRun>(
            "SELECT * FROM sync_runs ORDER BY started_at DESC LIMIT 1",
        )
        .fetch_optional(self.pool)
        .await
    }

    /// Recent runs, newest first.
    pub async fn recent(&self, limit: i64) -> Result<Vec<SyncRun>, sqlx::Error> {
        sqlx::query_as::<_, SyncRun>(
            "SELECT * FROM sync_runs ORDER BY started_at DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(self.pool)
        .await
    }
}
