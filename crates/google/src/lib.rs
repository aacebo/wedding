//! Google OAuth2 + read-only Gmail/Drive integration.
//!
//! Provides the OAuth2 authorization-code flow (Phase 02, with offline access so
//! we receive a refresh token), AES-GCM encryption for tokens at rest, and
//! read-only Gmail + Drive API clients (Phase 03) for ingestion.

mod crypto;
mod drive;
mod error;
mod gmail;
mod oauth;

pub use crypto::TokenCipher;
pub use drive::{DriveChanges, DriveClient, DriveFile};
pub use error::GoogleError;
pub use gmail::{GmailClient, GmailMessage};
pub use oauth::{DEFAULT_SCOPES, GoogleOAuth, TokenSet, UserInfo};
