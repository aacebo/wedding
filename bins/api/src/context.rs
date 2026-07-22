use chrono::{DateTime, Utc};
use sqlx::PgPool;

use admin::Allowlist;
use llm::OpenAiExtractor;
use storage::Storage;

use crate::google_auth::GoogleAuth;

#[derive(Clone)]
pub struct Context {
    pool: PgPool,
    start_time: DateTime<Utc>,
    allowlist: Allowlist,
    dev_login_enabled: bool,
    google: Option<GoogleAuth>,
    llm: Option<OpenAiExtractor>,
    llm_max_batch: i64,
}

impl Context {
    pub fn new(
        pool: PgPool,
        allowlist: Allowlist,
        dev_login_enabled: bool,
        google: Option<GoogleAuth>,
        llm: Option<OpenAiExtractor>,
        llm_max_batch: i64,
    ) -> Self {
        Self {
            pool,
            start_time: Utc::now(),
            allowlist,
            dev_login_enabled,
            google,
            llm,
            llm_max_batch,
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

    pub fn llm(&self) -> Option<&OpenAiExtractor> {
        self.llm.as_ref()
    }

    pub fn llm_max_batch(&self) -> i64 {
        self.llm_max_batch
    }

    pub fn storage(&self) -> Storage<'_> {
        Storage::new(&self.pool)
    }
}
