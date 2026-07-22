use std::env;

use admin::Allowlist;

/// Insecure default used only for local development when `SESSION_SECRET` is
/// unset. Must be at least 32 bytes for cookie key derivation. Never rely on
/// this in production — set a strong `SESSION_SECRET`.
const DEV_SESSION_SECRET: &str =
    "dev-only-insecure-session-secret-change-me-in-production-0123456789";

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    /// Emails permitted to access `/admin`.
    pub admin_allowlist: Allowlist,
    /// Secret used to sign the admin session cookie.
    pub session_secret: String,
    /// When true, enables the temporary `POST /admin/dev-login` endpoint used
    /// to establish a session before real Google SSO (Phase 02) lands.
    pub dev_login_enabled: bool,
    /// Whether the session cookie carries the `Secure` flag. Defaults to true
    /// (correct for the HTTPS production site); set `SESSION_COOKIE_SECURE=false`
    /// for local HTTP development.
    pub cookie_secure: bool,
}

impl Config {
    pub fn from_env() -> Self {
        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .expect("PORT must be a valid number");

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://admin:admin@localhost:5432/wedding".to_string());

        let admin_allowlist =
            Allowlist::from_csv(&env::var("ADMIN_ALLOWLIST").unwrap_or_default());

        let session_secret =
            env::var("SESSION_SECRET").unwrap_or_else(|_| DEV_SESSION_SECRET.to_string());

        let dev_login_enabled = env::var("ADMIN_DEV_LOGIN")
            .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE"))
            .unwrap_or(false);

        let cookie_secure = env::var("SESSION_COOKIE_SECURE")
            .map(|v| !matches!(v.as_str(), "0" | "false" | "FALSE"))
            .unwrap_or(true);

        Self {
            port,
            database_url,
            admin_allowlist,
            session_secret,
            dev_login_enabled,
            cookie_secure,
        }
    }
}
