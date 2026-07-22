use sqlx::PgPool;

use crate::types::GoogleAccount;

pub struct GoogleAccountStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> GoogleAccountStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get(&self, email: &str) -> Result<Option<GoogleAccount>, sqlx::Error> {
        sqlx::query_as::<_, GoogleAccount>("SELECT * FROM google_accounts WHERE email = $1")
            .bind(email)
            .fetch_optional(self.pool)
            .await
    }

    /// All linked accounts, used to fan out ingestion across both planners.
    pub async fn list(&self) -> Result<Vec<GoogleAccount>, sqlx::Error> {
        sqlx::query_as::<_, GoogleAccount>("SELECT * FROM google_accounts ORDER BY email")
            .fetch_all(self.pool)
            .await
    }

    /// Inserts or updates the account for `email`. When `refresh_token` is
    /// `None` (Google omits it on re-consent) any previously stored refresh
    /// token is preserved.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert(
        &self,
        email: &str,
        google_sub: Option<&str>,
        access_token: &[u8],
        refresh_token: Option<&[u8]>,
        token_expiry: Option<chrono::DateTime<chrono::Utc>>,
        scopes: &str,
    ) -> Result<GoogleAccount, sqlx::Error> {
        sqlx::query_as::<_, GoogleAccount>(
            r#"
            INSERT INTO google_accounts
                (email, google_sub, access_token, refresh_token, token_expiry, scopes, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (email) DO UPDATE SET
                google_sub    = EXCLUDED.google_sub,
                access_token  = EXCLUDED.access_token,
                refresh_token = COALESCE(EXCLUDED.refresh_token, google_accounts.refresh_token),
                token_expiry  = EXCLUDED.token_expiry,
                scopes        = EXCLUDED.scopes,
                updated_at    = NOW()
            RETURNING *
            "#,
        )
        .bind(email)
        .bind(google_sub)
        .bind(access_token)
        .bind(refresh_token)
        .bind(token_expiry)
        .bind(scopes)
        .fetch_one(self.pool)
        .await
    }

    /// Updates just the access token + expiry, e.g. after a refresh.
    pub async fn update_access(
        &self,
        email: &str,
        access_token: &[u8],
        token_expiry: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<GoogleAccount, sqlx::Error> {
        sqlx::query_as::<_, GoogleAccount>(
            r#"
            UPDATE google_accounts
            SET access_token = $2, token_expiry = $3, updated_at = NOW()
            WHERE email = $1
            RETURNING *
            "#,
        )
        .bind(email)
        .bind(access_token)
        .bind(token_expiry)
        .fetch_one(self.pool)
        .await
    }
}
