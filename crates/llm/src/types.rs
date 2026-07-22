//! Types crossing the extractor boundary.
//!
//! Dates are plain `Option<String>` (ISO `YYYY-MM-DD`) so this crate carries no
//! datetime dependency; the caller parses/validates them.

use serde::Deserialize;

/// The normalized communication handed to the model for extraction.
pub struct SourceInput {
    pub kind: String,
    pub sender: Option<String>,
    pub title: String,
    pub occurred_at: Option<String>,
    pub body: String,
}

/// A candidate task extracted from a source.
#[derive(Debug, Clone, Deserialize)]
pub struct ExtractedTodo {
    pub title: String,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default)]
    pub confidence: f64,
    #[serde(default)]
    pub notes: Option<String>,
}

/// A dated milestone (venue booked, tasting, etc.).
#[derive(Debug, Clone, Deserialize)]
pub struct ExtractedEvent {
    pub title: String,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub confidence: f64,
}

/// A hard due date the couple must not miss.
#[derive(Debug, Clone, Deserialize)]
pub struct ExtractedDeadline {
    pub title: String,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default = "default_severity")]
    pub severity: String,
    #[serde(default)]
    pub confidence: f64,
}

/// Token accounting returned alongside every extraction, for cost tracking.
#[derive(Debug, Clone, Default)]
pub struct Usage {
    pub model: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
}

/// The full structured result for one source.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Extraction {
    #[serde(default)]
    pub todos: Vec<ExtractedTodo>,
    #[serde(default, alias = "events")]
    pub timeline_events: Vec<ExtractedEvent>,
    #[serde(default)]
    pub deadlines: Vec<ExtractedDeadline>,
    #[serde(skip)]
    pub usage: Usage,
}

fn default_priority() -> String {
    "med".to_string()
}

fn default_severity() -> String {
    "med".to_string()
}
