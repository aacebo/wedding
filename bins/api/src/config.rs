use std::env;

use crate::admin_auth::AdminCredentials;

#[derive(Clone)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub admin_credentials: Option<AdminCredentials>,
    pub google_sheets_webhook_url: Option<String>,
    pub google_sheets_webhook_secret: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .expect("PORT must be a valid number");

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://admin:admin@localhost:5432/wedding".to_string());

        let admin_credentials = AdminCredentials::from_values(
            env::var("ADMIN_USERNAME").ok(),
            env::var("ADMIN_PASSWORD").ok(),
        );

        let google_sheets_webhook_url = env::var("GOOGLE_SHEETS_WEBHOOK_URL")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let google_sheets_webhook_secret = env::var("GOOGLE_SHEETS_WEBHOOK_SECRET")
            .ok()
            .filter(|value| !value.trim().is_empty());
        if google_sheets_webhook_url.is_some() != google_sheets_webhook_secret.is_some() {
            panic!("Set both GOOGLE_SHEETS_WEBHOOK_URL and GOOGLE_SHEETS_WEBHOOK_SECRET to enable Google Sheets sync");
        }

        Self {
            port,
            database_url,
            admin_credentials,
            google_sheets_webhook_url,
            google_sheets_webhook_secret,
        }
    }
}
