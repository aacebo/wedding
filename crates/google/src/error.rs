#[derive(thiserror::Error, Debug)]
pub enum GoogleError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("google api error ({0}): {1}")]
    Api(u16, String),
    #[error("crypto error: {0}")]
    Crypto(String),
}
