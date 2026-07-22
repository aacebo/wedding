use serde::Deserialize;
use url::Url;

use crate::error::GoogleError;

const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const USERINFO_ENDPOINT: &str = "https://www.googleapis.com/oauth2/v3/userinfo";

/// Read-only scopes requested at consent. `offline` access (see
/// [`GoogleOAuth::authorize_url`]) yields a refresh token so we can keep syncing
/// without the user present.
pub const DEFAULT_SCOPES: &[&str] = &[
    "openid",
    "email",
    "profile",
    "https://www.googleapis.com/auth/gmail.readonly",
    "https://www.googleapis.com/auth/drive.readonly",
];

/// Tokens returned by Google's token endpoint. Deliberately does not derive
/// `Debug` to avoid accidentally logging secrets.
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: i64,
    pub scope: String,
}

/// Identity of the signed-in Google user (from the userinfo endpoint).
#[derive(Debug, Deserialize)]
pub struct UserInfo {
    pub sub: String,
    pub email: String,
    #[serde(default)]
    pub email_verified: bool,
}

#[derive(Deserialize)]
struct RawToken {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    scope: Option<String>,
}

/// Minimal Google OAuth2 authorization-code client.
#[derive(Clone)]
pub struct GoogleOAuth {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    http: reqwest::Client,
}

impl GoogleOAuth {
    pub fn new(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            redirect_uri: redirect_uri.into(),
            http: reqwest::Client::new(),
        }
    }

    /// Builds the consent URL to redirect the user to. `state` is an opaque,
    /// unguessable value echoed back to the callback for CSRF protection.
    pub fn authorize_url(&self, state: &str) -> String {
        let scope = DEFAULT_SCOPES.join(" ");
        let mut url = Url::parse(AUTH_ENDPOINT).expect("valid auth endpoint");
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", &scope)
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent")
            .append_pair("include_granted_scopes", "true")
            .append_pair("state", state);
        url.into()
    }

    /// Exchanges an authorization code for tokens.
    pub async fn exchange_code(&self, code: &str) -> Result<TokenSet, GoogleError> {
        let params = [
            ("code", code),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("redirect_uri", self.redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ];
        let res = self.http.post(TOKEN_ENDPOINT).form(&params).send().await?;
        Self::parse_token(res).await
    }

    /// Uses a refresh token to obtain a fresh access token. Google does not
    /// return a new refresh token here, so callers keep the existing one.
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenSet, GoogleError> {
        let params = [
            ("refresh_token", refresh_token),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "refresh_token"),
        ];
        let res = self.http.post(TOKEN_ENDPOINT).form(&params).send().await?;
        Self::parse_token(res).await
    }

    /// Fetches the signed-in user's identity (sub + email).
    pub async fn fetch_userinfo(&self, access_token: &str) -> Result<UserInfo, GoogleError> {
        let res = self
            .http
            .get(USERINFO_ENDPOINT)
            .bearer_auth(access_token)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status().as_u16();
            return Err(GoogleError::Api(status, res.text().await.unwrap_or_default()));
        }

        Ok(res.json::<UserInfo>().await?)
    }

    async fn parse_token(res: reqwest::Response) -> Result<TokenSet, GoogleError> {
        if !res.status().is_success() {
            let status = res.status().as_u16();
            return Err(GoogleError::Api(status, res.text().await.unwrap_or_default()));
        }

        let raw = res.json::<RawToken>().await?;
        Ok(TokenSet {
            access_token: raw.access_token,
            refresh_token: raw.refresh_token,
            expires_in: raw.expires_in.unwrap_or(3600),
            scope: raw.scope.unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorize_url_has_required_params() {
        let oauth = GoogleOAuth::new("client-123", "secret", "https://baicebo.com/cb");
        let url = oauth.authorize_url("state-xyz");

        assert!(url.starts_with(AUTH_ENDPOINT));
        assert!(url.contains("client_id=client-123"));
        assert!(url.contains("access_type=offline"));
        assert!(url.contains("prompt=consent"));
        assert!(url.contains("state=state-xyz"));
        assert!(url.contains("gmail.readonly"));
        assert!(url.contains("drive.readonly"));
    }
}
