use std::{collections::HashMap, time::Duration};

use chrono::Utc;
use serde_json::{Value, json};
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct Rsvp<'a> {
    pub id: Uuid,
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub email: &'a str,
    pub phone: &'a str,
    pub accommodation: &'a str,
    pub dietary_requirements: &'a str,
    pub main_course: &'a str,
    pub attending: bool,
    pub invitation_code: &'a str,
    pub guest_count: u32,
}

#[derive(Debug)]
pub enum SaveError {
    Unavailable,
    Uncertain,
}

#[derive(Debug)]
pub enum GuestError {
    InvalidCode,
    Unavailable,
}

pub struct Notion {
    client: reqwest::Client,
    token: Option<String>,
    data_source: Option<String>,
    guests_data_source: Option<String>,
    guests_page: Option<String>,
    base_url: String,
    // Serialize submissions in this process to avoid double-click races.
    submissions: Mutex<HashMap<Uuid, bool>>,
}

fn rich_text(value: &str) -> Value {
    if value.is_empty() {
        json!({"rich_text": []})
    } else {
        json!({"rich_text": [{"text": {"content": value}}]})
    }
}

impl Notion {
    pub fn new(
        token: Option<String>,
        data_source: Option<String>,
        guests_data_source: Option<String>,
        guests_page: Option<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .no_proxy()
                .build()
                .expect("HTTP client"),
            token,
            data_source,
            guests_data_source,
            guests_page,
            base_url: "https://api.notion.com/v1".to_owned(),
            submissions: Mutex::new(HashMap::new()),
        }
    }

    #[cfg(test)]
    pub fn for_test(base_url: String) -> Self {
        let mut notion = Self::new(
            Some("test-token".into()),
            Some("test-source".into()),
            Some("test-guests".into()),
            Some("test-guests-page".into()),
        );
        notion.base_url = base_url;
        notion
    }

    fn configuration(&self) -> Result<(&str, &str, &str, &str), GuestError> {
        match (
            self.token.as_deref(),
            self.data_source.as_deref(),
            self.guests_data_source.as_deref(),
            self.guests_page.as_deref(),
        ) {
            (Some(token), Some(rsvps), Some(guests), Some(page)) => {
                Ok((token, rsvps, guests, page))
            }
            _ => {
                eprintln!(
                    "RSVP Notion configuration error: a Notion token or required data source is missing"
                );
                Err(GuestError::Unavailable)
            }
        }
    }

    pub async fn validate_rsvp_code(&self, code: &str) -> Result<(), GuestError> {
        let (token, _, guests, _) = self.configuration()?;
        let response = self
            .client
            .post(format!("{}/data_sources/{guests}/query", self.base_url))
            .bearer_auth(token)
            .header("Notion-Version", "2025-09-03")
            .json(&json!({
                "page_size": 1,
                "filter": {"property": "Invitation Code", "rich_text": {"equals": code}}
            }))
            .send()
            .await
            .map_err(|_| GuestError::Unavailable)?;
        if !response.status().is_success() {
            eprintln!("Guest-code lookup failed: HTTP {}", response.status());
            return Err(GuestError::Unavailable);
        }
        let result: Value = response.json().await.map_err(|_| GuestError::Unavailable)?;
        result
            .get("results")
            .and_then(Value::as_array)
            .and_then(|pages| pages.first())
            .ok_or(GuestError::InvalidCode)?;
        Ok(())
    }

    pub async fn save(&self, rsvp: &Rsvp<'_>) -> Result<(), SaveError> {
        let (token, source, _, _) = self.configuration().map_err(|_| SaveError::Unavailable)?;
        let mut submissions = self.submissions.lock().await;
        if let Some(confirmed) = submissions.get(&rsvp.id) {
            return if *confirmed {
                Ok(())
            } else {
                Err(SaveError::Uncertain)
            };
        }
        let query = self.client.post(format!("{}/data_sources/{source}/query", self.base_url))
            .bearer_auth(token).header("Notion-Version", "2025-09-03")
            .json(&json!({"page_size": 1, "filter": {"property": "Wedding Site RSVP ID", "rich_text": {"equals": rsvp.id.to_string()}}}))
            .send().await.map_err(|error| {
                eprintln!("RSVP Notion query failed: {}", if error.is_timeout() { "timed out" } else if error.is_connect() { "connection failed" } else { "request failed" });
                SaveError::Unavailable
            })?;
        if !query.status().is_success() {
            if query.status() == reqwest::StatusCode::NOT_FOUND {
                if let Ok(id) = Uuid::parse_str(source) {
                    eprintln!(
                        "RSVP Notion query returned 404 for data source {id}; check the configured ID and share the RSVP database with the integration used by NOTION_TOKEN"
                    );
                } else {
                    eprintln!(
                        "RSVP Notion query returned 404; NOTION_RSVP_DATA_SOURCE_ID is not a valid data source UUID"
                    );
                }
            } else {
                eprintln!("RSVP Notion query failed: HTTP {}", query.status());
            }
            return Err(SaveError::Unavailable);
        }
        let found: Value = query.json().await.map_err(|_| {
            eprintln!("RSVP Notion query failed: invalid response");
            SaveError::Unavailable
        })?;
        let results = found
            .get("results")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                eprintln!("RSVP Notion query failed: response missing results");
                SaveError::Unavailable
            })?;
        if !results.is_empty() {
            submissions.insert(rsvp.id, true);
            return Ok(());
        }
        let name = format!("{} {}", rsvp.first_name, rsvp.last_name);
        let main_course = if rsvp.attending {
            rsvp.main_course
        } else {
            "Not selected"
        };
        let mut body = json!({
            "properties": {
                "RSVP": {"title": [{"text": {"content": name}}]},
                "First Name": rich_text(rsvp.first_name),
                "Last Name": rich_text(rsvp.last_name),
                "Email": {"email": rsvp.email},
                "Phone": {"phone_number": if rsvp.phone.is_empty() { None } else { Some(rsvp.phone) }},
                "Accommodation": rich_text(rsvp.accommodation),
                "Dietary Requirements": rich_text(rsvp.dietary_requirements),
                "Main Course": {"select": {"name": main_course}},
                "RSVP Status": {"select": {"name": if rsvp.attending { "Confirmed" } else { "Declined" }}},
                "Guest Count": {"number": rsvp.guest_count},
                "Invitation Code": rich_text(rsvp.invitation_code),
                "Response Date": {"date": {"start": Utc::now().to_rfc3339()}},
                "Wedding Site RSVP ID": rich_text(&rsvp.id.to_string())
            }
        });
        body["parent"] = json!({"type": "data_source_id", "data_source_id": source});
        let result = self
            .client
            .post(format!("{}/pages", self.base_url))
            .bearer_auth(token)
            .header("Notion-Version", "2025-09-03")
            .json(&body)
            .send()
            .await;
        match result {
            Ok(response) if response.status().is_success() => {
                submissions.insert(rsvp.id, true);
                if let Err(error) = self.refresh_progress().await {
                    eprintln!("RSVP saved, but progress refresh failed: {error:?}");
                }
                Ok(())
            }
            Ok(response) if response.status().is_client_error() => {
                eprintln!("RSVP Notion create failed: HTTP {}", response.status());
                Err(SaveError::Unavailable)
            }
            result => {
                match result {
                    Ok(response) => eprintln!(
                        "RSVP Notion create outcome uncertain: HTTP {}",
                        response.status()
                    ),
                    Err(error) => eprintln!(
                        "RSVP Notion create outcome uncertain: {}",
                        if error.is_timeout() {
                            "timed out"
                        } else if error.is_connect() {
                            "connection failed"
                        } else {
                            "request failed"
                        }
                    ),
                }
                // The create may have succeeded; do not send it again in this process.
                submissions.insert(rsvp.id, false);
                Err(SaveError::Uncertain)
            }
        }
    }

    pub async fn refresh_progress(&self) -> Result<(), GuestError> {
        let (token, rsvps, guests, page) = self.configuration()?;
        let query = |source: &str| {
            self.client
                .post(format!("{}/data_sources/{source}/query", self.base_url))
                .bearer_auth(token)
                .header("Notion-Version", "2025-09-03")
                .json(&json!({"page_size": 100}))
                .send()
        };
        let guests: Value = query(guests)
            .await
            .map_err(|_| GuestError::Unavailable)?
            .error_for_status()
            .map_err(|_| GuestError::Unavailable)?
            .json()
            .await
            .map_err(|_| GuestError::Unavailable)?;
        let rsvps: Value = query(rsvps)
            .await
            .map_err(|_| GuestError::Unavailable)?
            .error_for_status()
            .map_err(|_| GuestError::Unavailable)?
            .json()
            .await
            .map_err(|_| GuestError::Unavailable)?;
        let number = |page: &Value, property: &str| {
            page.get("properties")
                .and_then(|properties| properties.get(property))
                .and_then(|value| value.get("number"))
                .and_then(Value::as_u64)
                .unwrap_or(0)
        };
        let total: u64 = guests
            .get("results")
            .and_then(Value::as_array)
            .ok_or(GuestError::Unavailable)?
            .iter()
            .map(|page| number(page, "Invited Guests"))
            .sum();
        let responded: u64 = rsvps
            .get("results")
            .and_then(Value::as_array)
            .ok_or(GuestError::Unavailable)?
            .iter()
            .map(|page| number(page, "Guest Count"))
            .sum();
        let percent = if total == 0 {
            0
        } else {
            responded.saturating_mul(100) / total
        };
        let filled = usize::try_from((percent + 2) / 5).unwrap_or(20).min(20);
        let progress = format!("{}{}", "█".repeat(filled), "░".repeat(20 - filled));
        let children: Value = self
            .client
            .get(format!(
                "{}/blocks/{page}/children?page_size=100",
                self.base_url
            ))
            .bearer_auth(token)
            .header("Notion-Version", "2025-09-03")
            .send()
            .await
            .map_err(|_| GuestError::Unavailable)?
            .error_for_status()
            .map_err(|_| GuestError::Unavailable)?
            .json()
            .await
            .map_err(|_| GuestError::Unavailable)?;
        let callout_id = children
            .get("results")
            .and_then(Value::as_array)
            .and_then(|blocks| {
                blocks
                    .iter()
                    .find(|block| block.get("type").and_then(Value::as_str) == Some("callout"))
            })
            .and_then(|block| block.get("id"))
            .and_then(Value::as_str)
            .ok_or(GuestError::Unavailable)?;
        // The callout is reserved for the generated summary. Remove any old child blocks
        // before updating its text so a prior summary can never remain underneath it.
        let callout_children: Value = self
            .client
            .get(format!(
                "{}/blocks/{callout_id}/children?page_size=100",
                self.base_url
            ))
            .bearer_auth(token)
            .header("Notion-Version", "2025-09-03")
            .send()
            .await
            .map_err(|_| GuestError::Unavailable)?
            .error_for_status()
            .map_err(|_| GuestError::Unavailable)?
            .json()
            .await
            .map_err(|_| GuestError::Unavailable)?;
        let child_ids = callout_children
            .get("results")
            .and_then(Value::as_array)
            .ok_or(GuestError::Unavailable)?
            .iter()
            .filter_map(|block| block.get("id").and_then(Value::as_str));
        for child_id in child_ids {
            self.client
                .delete(format!("{}/blocks/{child_id}", self.base_url))
                .bearer_auth(token)
                .header("Notion-Version", "2025-09-03")
                .send()
                .await
                .map_err(|_| GuestError::Unavailable)?
                .error_for_status()
                .map_err(|_| GuestError::Unavailable)?;
        }
        self.client
            .patch(format!("{}/blocks/{callout_id}", self.base_url))
            .bearer_auth(token)
            .header("Notion-Version", "2025-09-03")
            .json(&json!({"callout": {"rich_text": [{"type": "text", "text": {"content": format!("RSVP progress — {percent}% responded\n\n{progress}  {percent}%\n\n{responded} of {total} invited guests have responded")}, "annotations": {"bold": true}}]}}))
            .send().await.map_err(|_| GuestError::Unavailable)?
            .error_for_status().map_err(|_| GuestError::Unavailable)?;
        Ok(())
    }
}
