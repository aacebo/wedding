/// A linked Google account and its encrypted OAuth tokens.
///
/// `access_token` / `refresh_token` are AES-GCM ciphertext (see the `google`
/// crate's `TokenCipher`), never plaintext. This type intentionally does not
/// derive `Serialize` or `Debug` to avoid leaking token material.
#[derive(Clone, sqlx::FromRow)]
pub struct GoogleAccount {
    pub id: uuid::Uuid,
    pub email: String,
    pub google_sub: Option<String>,
    pub access_token: Vec<u8>,
    pub refresh_token: Option<Vec<u8>>,
    pub token_expiry: Option<chrono::DateTime<chrono::Utc>>,
    pub scopes: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
