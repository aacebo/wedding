mod allowlist;

pub use allowlist::*;

/// Session key under which the authenticated admin's email is stored.
///
/// Shared between the login flow (which writes it) and the `AdminSession`
/// extractor (which reads it) so the two never drift apart.
pub const SESSION_EMAIL_KEY: &str = "admin_email";
