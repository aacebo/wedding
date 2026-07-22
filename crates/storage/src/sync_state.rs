use sqlx::PgPool;

use crate::types::SyncState;

pub struct SyncStateStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> SyncStateStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get(
        &self,
        account_email: &str,
        provider: &str,
    ) -> Result<Option<SyncState>, sqlx::Error> {
        sqlx::query_as::<_, SyncState>(
            "SELECT * FROM sync_state WHERE account_email = $1 AND provider = $2",
        )
        .bind(account_email)
        .bind(provider)
        .fetch_optional(self.pool)
        .await
    }

    pub async fn list(&self) -> Result<Vec<SyncState>, sqlx::Error> {
        sqlx::query_as::<_, SyncState>(
            "SELECT * FROM sync_state ORDER BY account_email, provider",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Records the latest cursor + sync time for an account/provider.
    pub async fn upsert(
        &self,
        account_email: &str,
        provider: &str,
        cursor: Option<&str>,
    ) -> Result<SyncState, sqlx::Error> {
        sqlx::query_as::<_, SyncState>(
            r#"
            INSERT INTO sync_state (account_email, provider, cursor, last_synced_at)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (account_email, provider) DO UPDATE SET
                cursor         = EXCLUDED.cursor,
                last_synced_at = NOW()
            RETURNING *
            "#,
        )
        .bind(account_email)
        .bind(provider)
        .bind(cursor)
        .fetch_one(self.pool)
        .await
    }
}
