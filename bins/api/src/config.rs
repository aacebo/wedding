use std::env;

#[derive(Clone)]
pub struct Config {
    pub port: u16,
    pub notion_token: Option<String>,
    pub notion_data_source_id: Option<String>,
    pub notion_guests_data_source_id: Option<String>,
    pub notion_guests_page_id: Option<String>,
    pub notion_progress_webhook_secret: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_owned())
                .parse()
                .expect("PORT must be a valid number"),
            notion_token: env::var("NOTION_TOKEN").ok().filter(|s| !s.is_empty()),
            notion_data_source_id: env::var("NOTION_RSVP_DATA_SOURCE_ID")
                .ok()
                .filter(|s| !s.is_empty()),
            notion_guests_data_source_id: Some(
                env::var("NOTION_GUESTS_DATA_SOURCE_ID")
                    .unwrap_or_else(|_| "41b96eec-0111-81e1-a385-000bebe7a8ad".to_owned()),
            ),
            notion_guests_page_id: Some(
                env::var("NOTION_GUESTS_PAGE_ID")
                    .unwrap_or_else(|_| "41b96eec-0111-811a-b80b-fa7bb1979db2".to_owned()),
            ),
            notion_progress_webhook_secret: env::var("NOTION_PROGRESS_WEBHOOK_SECRET")
                .ok()
                .filter(|s| !s.is_empty()),
        }
    }
}
