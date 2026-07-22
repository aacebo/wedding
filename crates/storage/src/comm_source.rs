use sqlx::PgPool;

use crate::types::{CommSource, NewCommSource};

pub struct CommSourceStorage<'a> {
    pool: &'a PgPool,
}

/// Outcome of an upsert: whether the row was newly inserted or updated in place.
pub enum UpsertOutcome {
    Inserted,
    Updated,
}

impl<'a> CommSourceStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Inserts a source, or updates the existing row with the same
    /// `(provider, external_id)`. Returns whether it was an insert or update so
    /// callers can report new-vs-updated counts. Re-processing is reset
    /// (`processed_at = NULL`) whenever content changes so Phase 04 re-examines it.
    pub async fn upsert(&self, s: &NewCommSource) -> Result<UpsertOutcome, sqlx::Error> {
        let inserted: bool = sqlx::query_scalar(
            r#"
            INSERT INTO comm_sources
                (account_email, provider, external_id, kind, sender, title,
                 snippet, body_text, url, occurred_at, raw, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())
            ON CONFLICT (provider, external_id) DO UPDATE SET
                account_email = EXCLUDED.account_email,
                kind          = EXCLUDED.kind,
                sender        = EXCLUDED.sender,
                title         = EXCLUDED.title,
                snippet       = EXCLUDED.snippet,
                body_text     = EXCLUDED.body_text,
                url           = EXCLUDED.url,
                occurred_at   = EXCLUDED.occurred_at,
                raw           = EXCLUDED.raw,
                processed_at  = NULL,
                updated_at    = NOW()
            RETURNING (xmax = 0) AS inserted
            "#,
        )
        .bind(&s.account_email)
        .bind(&s.provider)
        .bind(&s.external_id)
        .bind(&s.kind)
        .bind(&s.sender)
        .bind(&s.title)
        .bind(&s.snippet)
        .bind(&s.body_text)
        .bind(&s.url)
        .bind(s.occurred_at)
        .bind(&s.raw)
        .fetch_one(self.pool)
        .await?;

        Ok(if inserted {
            UpsertOutcome::Inserted
        } else {
            UpsertOutcome::Updated
        })
    }

    /// Marks a Drive source removed upstream; safe no-op if we never saw it.
    pub async fn mark_removed(&self, provider: &str, external_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM comm_sources WHERE provider = $1 AND external_id = $2",
        )
        .bind(provider)
        .bind(external_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Sources not yet processed by the extraction pipeline (Phase 04 worklist).
    pub async fn unprocessed(&self, limit: i64) -> Result<Vec<CommSource>, sqlx::Error> {
        sqlx::query_as::<_, CommSource>(
            "SELECT * FROM comm_sources WHERE processed_at IS NULL ORDER BY occurred_at DESC NULLS LAST LIMIT $1",
        )
        .bind(limit)
        .fetch_all(self.pool)
        .await
    }

    /// Stamps a source as processed so the extraction pipeline skips it until its
    /// content changes (an upsert resets `processed_at` to NULL).
    pub async fn mark_processed(&self, id: uuid::Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE comm_sources SET processed_at = NOW(), updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    pub async fn count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM comm_sources")
            .fetch_one(self.pool)
            .await
    }

    /// Most-recent sources for the inbox view, optionally filtered by provider
    /// (`gmail` / `drive`). A `None` provider returns all.
    pub async fn recent(
        &self,
        provider: Option<&str>,
        limit: i64,
    ) -> Result<Vec<CommSource>, sqlx::Error> {
        match provider {
            Some(p) => {
                sqlx::query_as::<_, CommSource>(
                    "SELECT * FROM comm_sources WHERE provider = $1 ORDER BY occurred_at DESC NULLS LAST LIMIT $2",
                )
                .bind(p)
                .bind(limit)
                .fetch_all(self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, CommSource>(
                    "SELECT * FROM comm_sources ORDER BY occurred_at DESC NULLS LAST LIMIT $1",
                )
                .bind(limit)
                .fetch_all(self.pool)
                .await
            }
        }
    }
}
