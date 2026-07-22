/// The set of email addresses permitted to access the hidden `/admin` area.
///
/// Emails are normalized (trimmed + lowercased) on construction and comparison
/// so allowlist checks are case-insensitive and whitespace-tolerant.
#[derive(Debug, Clone, Default)]
pub struct Allowlist {
    emails: Vec<String>,
}

impl Allowlist {
    pub fn new(emails: impl IntoIterator<Item = String>) -> Self {
        let emails = emails
            .into_iter()
            .map(|e| e.trim().to_lowercase())
            .filter(|e| !e.is_empty())
            .collect();

        Self { emails }
    }

    /// Build an allowlist from a comma-separated string (e.g. an env var).
    pub fn from_csv(csv: &str) -> Self {
        Self::new(csv.split(',').map(str::to_string))
    }

    /// Whether the given email is permitted. Case-insensitive.
    pub fn contains(&self, email: &str) -> bool {
        let email = email.trim().to_lowercase();
        self.emails.contains(&email)
    }

    pub fn is_empty(&self) -> bool {
        self.emails.is_empty()
    }

    pub fn emails(&self) -> &[String] {
        &self.emails
    }
}
