use chrono::{DateTime, Utc};
use sqlx::PgPool;

use admin::Allowlist;
use storage::Storage;

use crate::google_auth::GoogleAuth;

#[derive(Clone)]
pub struct Context {
    pool: PgPool,
    start_time: DateTime<Utc>,
    allowlist: Allowlist,
    dev_login_enabled: bool,
    google: Option<GoogleAuth>,
}

impl Context {
    pub fn new(
        pool: PgPool,
        allowlist: Allowlist,
        dev_login_enabled: bool,
        google: Option<GoogleAuth>,
    ) -> Self {
        Self {
            pool,
            start_time: Utc::now(),
            allowlist,
            dev_login_enabled,
            google,
        }
    }

    pub fn start_time(&self) -> DateTime<Utc> {
        self.start_time
    }

    pub fn uptime(&self) -> chrono::Duration {
        Utc::now() - self.start_time
    }

    pub fn allowlist(&self) -> &Allowlist {
        &self.allowlist
    }

    pub fn dev_login_enabled(&self) -> bool {
        self.dev_login_enabled
    }

    pub fn google(&self) -> Option<&GoogleAuth> {
        self.google.as_ref()
    }

    pub fn storage(&self) -> Storage<'_> {
        Storage::new(&self.pool)
    }
}
