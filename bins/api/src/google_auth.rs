use chrono::{Duration, Utc};

use google::{GoogleOAuth, TokenCipher, TokenSet, UserInfo};
use storage::Storage;

/// Refresh access tokens this many seconds before they actually expire, to
/// avoid racing the expiry on a slow request.
const EXPIRY_SKEW_SECS: i64 = 60;

#[derive(thiserror::Error, Debug)]
pub enum GoogleAuthError {
    #[error(transparent)]
    Google(#[from] google::GoogleError),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error("no linked google account for {0}")]
    NotLinked(String),
    #[error("no refresh token stored for {0}")]
    NoRefresh(String),
}

/// Ties the stateless OAuth client together with token encryption and storage:
/// completes logins, persists encrypted tokens, and hands out valid access
/// tokens (refreshing transparently when expired).
#[derive(Clone)]
pub struct GoogleAuth {
    oauth: GoogleOAuth,
    cipher: TokenCipher,
}

impl GoogleAuth {
    pub fn new(
        client_id: String,
        client_secret: String,
        redirect_uri: String,
        encryption_key: &str,
    ) -> Self {
        Self {
            oauth: GoogleOAuth::new(client_id, client_secret, redirect_uri),
            cipher: TokenCipher::new(encryption_key),
        }
    }

    pub fn authorize_url(&self, state: &str) -> String {
        self.oauth.authorize_url(state)
    }

    /// Exchanges the authorization code for tokens and the user's identity.
    /// Nothing is persisted here so the caller can enforce the allowlist
    /// *before* any tokens touch the database.
    pub async fn exchange(&self, code: &str) -> Result<(UserInfo, TokenSet), GoogleAuthError> {
        let tokens = self.oauth.exchange_code(code).await?;
        let info = self.oauth.fetch_userinfo(&tokens.access_token).await?;
        Ok((info, tokens))
    }

    /// Encrypts and stores the tokens for an (already allowlist-checked) user.
    pub async fn persist(
        &self,
        storage: &Storage<'_>,
        info: &UserInfo,
        tokens: &TokenSet,
    ) -> Result<(), GoogleAuthError> {
        let enc_access = self.cipher.encrypt(&tokens.access_token)?;
        let enc_refresh = tokens
            .refresh_token
            .as_ref()
            .map(|r| self.cipher.encrypt(r))
            .transpose()?;
        let expiry = Utc::now() + Duration::seconds(tokens.expires_in);

        storage
            .google()
            .upsert(
                &info.email,
                Some(&info.sub),
                &enc_access,
                enc_refresh.as_deref(),
                Some(expiry),
                &tokens.scope,
            )
            .await?;

        Ok(())
    }

    /// Returns a currently-valid access token for `email`, refreshing and
    /// re-persisting it if the stored one has expired (or is about to).
    pub async fn valid_access_token(
        &self,
        storage: &Storage<'_>,
        email: &str,
    ) -> Result<String, GoogleAuthError> {
        let account = storage
            .google()
            .get(email)
            .await?
            .ok_or_else(|| GoogleAuthError::NotLinked(email.to_string()))?;

        let still_valid = account
            .token_expiry
            .map(|expiry| expiry > Utc::now() + Duration::seconds(EXPIRY_SKEW_SECS))
            .unwrap_or(false);

        if still_valid {
            return Ok(self.cipher.decrypt(&account.access_token)?);
        }

        let enc_refresh = account
            .refresh_token
            .ok_or_else(|| GoogleAuthError::NoRefresh(email.to_string()))?;
        let refresh = self.cipher.decrypt(&enc_refresh)?;
        let tokens = self.oauth.refresh_token(&refresh).await?;

        let enc_access = self.cipher.encrypt(&tokens.access_token)?;
        let expiry = Utc::now() + Duration::seconds(tokens.expires_in);
        storage
            .google()
            .update_access(email, &enc_access, Some(expiry))
            .await?;

        Ok(tokens.access_token)
    }
}
