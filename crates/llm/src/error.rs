#[derive(thiserror::Error, Debug)]
pub enum LlmError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    /// Non-2xx from the OpenAI API: status code + response body.
    #[error("openai api error {0}: {1}")]
    Api(u16, String),
    /// The model returned content that could not be parsed into our schema.
    #[error("failed to parse model output: {0}")]
    Parse(String),
}
