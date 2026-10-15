use actix_web::{
    Error, HttpResponse, error::ErrorInternalServerError, get, http::StatusCode, post, web,
};
use askama::Template;
use storage::SubmitRsvpError;

use super::PageMetadata;
use crate::RequestContext;
use crate::rsvp_validation::{valid_email, valid_name, valid_party_size};

#[derive(Default, serde::Deserialize)]
#[serde(default)]
pub(crate) struct RsvpForm {
    name: String,
    email: String,
    attending: String,
    party_size: String,
}

#[derive(Default)]
struct FormErrors {
    name: &'static str,
    email: &'static str,
    attending: &'static str,
    party_size: &'static str,
    general: &'static str,
}

impl FormErrors {
    fn is_empty(&self) -> bool {
        self.name.is_empty()
            && self.email.is_empty()
            && self.attending.is_empty()
            && self.party_size.is_empty()
            && self.general.is_empty()
    }
}

impl RsvpForm {
    fn validate(&mut self) -> Result<(bool, i16), FormErrors> {
        self.name = self.name.trim().to_owned();
        self.email = self.email.trim().to_lowercase();
        let mut errors = FormErrors::default();
        if !valid_name(&self.name) {
            errors.name = "Enter your name (up to 200 characters, without control characters).";
        }
        if !valid_email(&self.email) {
            errors.email = "Enter a valid email address.";
        }
        let attending = match self.attending.as_str() {
            "yes" => true,
            "no" => false,
            _ => {
                errors.attending = "Choose whether you will attend.";
                false
            }
        };
        let party_size = if !attending && matches!(self.party_size.as_str(), "" | "0" | "1" | "2") {
            0
        } else {
            match self.party_size.parse::<i16>() {
                Ok(size) if valid_party_size(attending, size) => size,
                _ => {
                    errors.party_size = "Choose 1 or 2 people if attending; declines count as 0.";
                    0
                }
            }
        };
        if errors.is_empty() {
            Ok((attending, party_size))
        } else {
            Err(errors)
        }
    }
}

#[derive(Template)]
#[template(path = "rsvp.html")]
struct RsvpPage {
    meta: PageMetadata,
    form: RsvpForm,
    errors: FormErrors,
    confirmation: bool,
}

fn render(
    form: RsvpForm,
    errors: FormErrors,
    confirmation: bool,
    status: StatusCode,
) -> Result<HttpResponse, Error> {
    let page = RsvpPage {
        meta: PageMetadata::new(
            "RSVP - Nancy & Alexander",
            "Let us know whether you will join our wedding celebration in Tuscany.",
            "https://baicebo.com/rsvp",
        ),
        form,
        errors,
        confirmation,
    };
    Ok(HttpResponse::build(status)
        .insert_header(("Cache-Control", "no-store"))
        .content_type("text/html; charset=utf-8")
        .body(page.render().map_err(ErrorInternalServerError)?))
}

#[get("/rsvp")]
pub async fn get() -> Result<HttpResponse, Error> {
    render(
        RsvpForm {
            party_size: "1".into(),
            ..Default::default()
        },
        FormErrors::default(),
        false,
        StatusCode::OK,
    )
}

#[get("/rsvp/thanks")]
pub async fn thanks() -> Result<HttpResponse, Error> {
    render(
        RsvpForm::default(),
        FormErrors::default(),
        true,
        StatusCode::OK,
    )
}

#[post("/rsvp")]
pub async fn post(ctx: RequestContext, form: web::Form<RsvpForm>) -> Result<HttpResponse, Error> {
    let mut form = form.into_inner();
    let (attending, party_size) = match form.validate() {
        Ok(values) => values,
        Err(errors) => return render(form, errors, false, StatusCode::UNPROCESSABLE_ENTITY),
    };
    match ctx
        .storage()
        .rsvps()
        .submit(&form.name, &form.email, attending, party_size)
        .await
    {
        Ok(()) => Ok(HttpResponse::SeeOther()
            .insert_header(("Location", "/rsvp/thanks"))
            .insert_header(("Cache-Control", "no-store"))
            .finish()),
        Err(SubmitRsvpError::Duplicate) => render(
            form,
            FormErrors {
                general: "An RSVP has already been received for this email. Your original response has not been changed. Please contact us to make changes.",
                ..Default::default()
            },
            false,
            StatusCode::CONFLICT,
        ),
        Err(SubmitRsvpError::Database(error)) => {
            eprintln!(
                "RSVP submission storage error [{}]: {error}",
                ctx.request_id()
            );
            render(
                form,
                FormErrors {
                    general: "We could not save your RSVP. Please try again or contact us.",
                    ..Default::default()
                },
                false,
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form(attending: &str, size: &str) -> RsvpForm {
        RsvpForm {
            name: " A Guest ".into(),
            email: " GUEST@example.com ".into(),
            attending: attending.into(),
            party_size: size.into(),
        }
    }

    #[test]
    fn validates_attendance_and_normalizes_identity() {
        let mut values = form("yes", "2");
        assert_eq!(values.validate().ok(), Some((true, 2)));
        assert_eq!(values.name, "A Guest");
        assert_eq!(values.email, "guest@example.com");
        assert_eq!(form("no", "2").validate().ok(), Some((false, 0)));
        for (attendance, size) in [
            ("", "1"),
            ("maybe", "1"),
            ("yes", "0"),
            ("yes", "3"),
            ("yes", ""),
            ("no", "3"),
        ] {
            assert!(form(attendance, size).validate().is_err());
        }
    }
}
