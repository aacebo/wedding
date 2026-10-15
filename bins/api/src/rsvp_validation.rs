pub fn valid_party_size(attending: bool, party_size: i16) -> bool {
    if attending {
        (1..=2).contains(&party_size)
    } else {
        party_size == 0
    }
}

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
    fn attendance_party_size_boundaries() {
        for size in [-1, 0, 1, 2, 3, i16::MAX] {
            assert_eq!(valid_party_size(true, size), size == 1 || size == 2);
            assert_eq!(valid_party_size(false, size), size == 0);
        }
    }

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
