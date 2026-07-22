//! Google OAuth2 + read-only Gmail/Drive integration.
//!
//! Phase 02 provides the OAuth2 authorization-code flow (with offline access so
//! we receive a refresh token) and AES-GCM encryption for tokens at rest. Gmail
//! and Drive API clients are added in Phase 03.

mod crypto;
mod error;
mod oauth;

pub use crypto::TokenCipher;
pub use error::GoogleError;
pub use oauth::{DEFAULT_SCOPES, GoogleOAuth, TokenSet, UserInfo};
