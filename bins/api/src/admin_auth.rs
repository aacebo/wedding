use actix_web::{Error, dev::ServiceRequest, web};
use actix_web_httpauth::extractors::AuthenticationError;
use actix_web_httpauth::extractors::basic::{BasicAuth, Config};
use subtle::ConstantTimeEq;

#[derive(Clone)]
pub struct AdminCredentials {
    username: String,
    password: String,
}

impl AdminCredentials {
    pub fn from_values(username: Option<String>, password: Option<String>) -> Option<Self> {
        match (username, password) {
            (None, None) => None,
            (Some(username), Some(password))
                if !username.trim().is_empty()
                    && !username.contains(':')
                    && !password.trim().is_empty() =>
            {
                Some(Self { username, password })
            }
            _ => panic!(
                "Set both ADMIN_USERNAME and ADMIN_PASSWORD to nonempty values; ADMIN_USERNAME must not contain ':'"
            ),
        }
    }

    fn matches(&self, auth: &BasicAuth) -> bool {
        let username = self.username.as_bytes().ct_eq(auth.user_id().as_bytes());
        let password = self
            .password
            .as_bytes()
            .ct_eq(auth.password().unwrap_or_default().as_bytes());
        bool::from(username & password)
    }
}

pub struct AdminAccess(pub Option<AdminCredentials>);

pub async fn validate(
    request: ServiceRequest,
    auth: BasicAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let allowed = request
        .app_data::<web::Data<AdminAccess>>()
        .and_then(|access| access.0.as_ref())
        .is_some_and(|credentials| credentials.matches(&auth));
    if allowed {
        Ok(request)
    } else {
        let error = AuthenticationError::from(Config::default().realm("Wedding admin"));
        Err((error.into(), request))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_configuration_disables_admin() {
        assert!(AdminCredentials::from_values(None, None).is_none());
    }

    #[test]
    #[should_panic(expected = "Set both ADMIN_USERNAME and ADMIN_PASSWORD")]
    fn partial_configuration_is_rejected() {
        AdminCredentials::from_values(Some("admin".into()), None);
    }

    #[test]
    #[should_panic(expected = "Set both ADMIN_USERNAME and ADMIN_PASSWORD")]
    fn empty_configuration_is_rejected() {
        AdminCredentials::from_values(Some("admin".into()), Some(" ".into()));
    }

    #[test]
    #[should_panic(expected = "ADMIN_USERNAME must not contain ':'")]
    fn colon_in_username_is_rejected() {
        AdminCredentials::from_values(Some("admin:name".into()), Some("password".into()));
    }
}
