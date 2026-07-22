use crate::error::LlmError;
use crate::types::{Extraction, SourceInput};

/// Provider-agnostic extraction interface so the OpenAI backend can be swapped
/// (e.g. for a local model or a test double) without touching the service layer.
#[allow(async_fn_in_trait)]
pub trait LlmExtractor {
    /// Extracts todos, timeline events, and deadlines from a single source.
    async fn extract(&self, input: &SourceInput) -> Result<Extraction, LlmError>;
}
