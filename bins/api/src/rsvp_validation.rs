pub fn valid_name(name: &str) -> bool {
    !name.trim().is_empty() && name.chars().count() <= 200 && !name.chars().any(char::is_control)
}

pub fn valid_email(email: &str) -> bool {
    email.len() <= 254 && email_address::EmailAddress::is_valid(email)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_validation() {
        assert!(valid_name("A Guest"));
        assert!(!valid_name("   "));
        assert!(!valid_name("A\nGuest"));
        assert!(!valid_name(&"a".repeat(201)));
        assert!(valid_name(&"é".repeat(200)));
        assert!(valid_email("guest@example.com"));
        for email in ["", "guest", "guest@", "@example.com", "a b@example.com"] {
            assert!(!valid_email(email));
        }
        assert!(!valid_email(&format!("{}@example.com", "a".repeat(255))));
    }
}
