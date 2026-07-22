use serde::Deserialize;

use crate::error::LlmError;
use crate::extractor::LlmExtractor;
use crate::prompt::{SYSTEM_PROMPT, build_user_message};
use crate::types::{Extraction, SourceInput, Usage};

const CHAT_ENDPOINT: &str = "https://api.openai.com/v1/chat/completions";
/// Guard against pathologically large documents blowing up the prompt (and cost).
const MAX_BODY_CHARS: usize = 24_000;

/// OpenAI-backed [`LlmExtractor`]. Uses Chat Completions with JSON-object
/// response format so the model returns parseable structured output.
#[derive(Clone)]
pub struct OpenAiExtractor {
    http: reqwest::Client,
    api_key: String,
    model: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<ApiUsage>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Deserialize)]
struct ApiUsage {
    #[serde(default)]
    prompt_tokens: i32,
    #[serde(default)]
    completion_tokens: i32,
}

impl OpenAiExtractor {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

impl LlmExtractor for OpenAiExtractor {
    async fn extract(&self, input: &SourceInput) -> Result<Extraction, LlmError> {
        let body: String = input.body.chars().take(MAX_BODY_CHARS).collect();
        let trimmed = SourceInput {
            kind: input.kind.clone(),
            sender: input.sender.clone(),
            title: input.title.clone(),
            occurred_at: input.occurred_at.clone(),
            body,
        };

        let payload = serde_json::json!({
            "model": self.model,
            "temperature": 0,
            "response_format": { "type": "json_object" },
            "messages": [
                { "role": "system", "content": SYSTEM_PROMPT },
                { "role": "user", "content": build_user_message(&trimmed) }
            ]
        });

        let res = self
            .http
            .post(CHAT_ENDPOINT)
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status().as_u16();
            return Err(LlmError::Api(status, res.text().await.unwrap_or_default()));
        }

        let chat: ChatResponse = res.json().await?;
        let content = chat
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or_else(|| LlmError::Parse("no content in response".to_string()))?;

        let mut extraction: Extraction =
            serde_json::from_str(&content).map_err(|e| LlmError::Parse(e.to_string()))?;

        extraction.usage = Usage {
            model: self.model.clone(),
            prompt_tokens: chat.usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0),
            completion_tokens: chat.usage.map(|u| u.completion_tokens).unwrap_or(0),
        };

        Ok(extraction)
    }
}
