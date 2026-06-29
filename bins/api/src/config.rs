use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .expect("PORT must be a valid number");

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://admin:admin@localhost:5432/wedding".to_string());

        Self { port, database_url }
    }
}
