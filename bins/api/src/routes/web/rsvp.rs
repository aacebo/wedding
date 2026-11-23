use crate::notion::{GuestError, Notion, Rsvp, SaveError};
use actix_web::{
    Error, HttpResponse, error::ErrorInternalServerError, get, http::StatusCode, post, web,
};
use askama::Template;

use super::PageMetadata;
use crate::rsvp_validation::{valid_email, valid_name};

#[derive(Default, serde::Deserialize)]
#[serde(default)]
pub(crate) struct RsvpForm {
    submission_id: String,
    invitation_code: String,
    first_name: String,
    last_name: String,
    email: String,
    phone: String,
    accommodation: String,
    dietary_requirements: String,
    main_course: String,
}

#[derive(Default)]
struct FormErrors {
    code: &'static str,
    name: &'static str,
    email: &'static str,
    phone: &'static str,
    accommodation: &'static str,
    dietary_requirements: &'static str,
    main_course: &'static str,
    general: &'static str,
}

impl FormErrors {
    fn is_empty(&self) -> bool {
        self.code.is_empty()
            && self.name.is_empty()
            && self.email.is_empty()
            && self.phone.is_empty()
            && self.accommodation.is_empty()
            && self.dietary_requirements.is_empty()
            && self.main_course.is_empty()
            && self.general.is_empty()
    }
}

impl RsvpForm {
    fn validate(&mut self) -> Result<(), FormErrors> {
        self.invitation_code = self.invitation_code.trim().to_owned();
        self.first_name = self.first_name.trim().to_owned();
        self.last_name = self.last_name.trim().to_owned();
        self.email = self.email.trim().to_lowercase();
        let mut errors = FormErrors::default();
        self.invitation_code.make_ascii_uppercase();
        if self.invitation_code != "BAICEBO" {
            errors.code = "Enter the invitation code from your invitation.";
        }
        if !valid_name(&self.first_name)
            || !valid_name(&self.last_name)
            || self.first_name.len() + self.last_name.len() > 199
            || [self.first_name.as_str(), self.last_name.as_str()]
                .iter()
                .any(|name| {
                    name.contains(['&', '+'])
                        || name
                            .split_whitespace()
                            .any(|word| word.eq_ignore_ascii_case("family"))
                })
        {
            errors.name = "Enter your individual first and last name.";
        }
        if !valid_email(&self.email) {
            errors.email = "Enter a valid email address.";
        }
        if self.phone.trim().is_empty()
            || self.phone.chars().count() > 100
            || self.phone.chars().any(char::is_control)
        {
            errors.phone = "Enter your phone number for transportation.";
        }
        if self.accommodation.chars().count() > 1000
            || self.accommodation.chars().any(char::is_control)
        {
            errors.accommodation =
                "Enter accommodation without control characters (up to 1,000 characters).";
        }
        if self.dietary_requirements.chars().count() > 1000
            || self.dietary_requirements.chars().any(char::is_control)
        {
            errors.dietary_requirements =
                "Enter dietary requirements without control characters (up to 1,000 characters).";
        }
        if !matches!(
            self.main_course.as_str(),
            "Beef" | "Sea Bass" | "Not selected"
        ) {
            errors.main_course = "Choose your main course or request a dietary alternative.";
        }
        if errors.is_empty() {
            Ok(())
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
            submission_id: uuid::Uuid::new_v4().to_string(),
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
pub async fn post(
    notion: web::Data<Notion>,
    form: web::Form<RsvpForm>,
) -> Result<HttpResponse, Error> {
    let mut form = form.into_inner();
    match form.validate() {
        Ok(()) => {}
        Err(errors) => return render(form, errors, false, StatusCode::UNPROCESSABLE_ENTITY),
    }
    match notion.validate_rsvp_code(&form.invitation_code).await {
        Ok(()) => {}
        Err(GuestError::InvalidCode) => {
            return render(
                form,
                FormErrors {
                    code: "That invitation code is not valid.",
                    ..Default::default()
                },
                false,
                StatusCode::UNPROCESSABLE_ENTITY,
            );
        }
        Err(GuestError::Unavailable) => {
            return render(
                form,
                FormErrors {
                    general: "We could not verify your invitation code. Please try again shortly.",
                    ..Default::default()
                },
                false,
                StatusCode::SERVICE_UNAVAILABLE,
            );
        }
    };
    let id = match uuid::Uuid::parse_str(&form.submission_id) {
        Ok(id) => id,
        Err(_) => {
            return render(
                form,
                FormErrors {
                    general: "Please reload the RSVP page and try again.",
                    ..Default::default()
                },
                false,
                StatusCode::BAD_REQUEST,
            );
        }
    };
    match notion
        .save(&Rsvp {
            id,
            first_name: &form.first_name,
            last_name: &form.last_name,
            email: &form.email,
            phone: form.phone.trim(),
            accommodation: form.accommodation.trim(),
            dietary_requirements: form.dietary_requirements.trim(),
            main_course: form.main_course.trim(),
            attending: true,
            invitation_code: &form.invitation_code,
            guest_count: 1,
        })
        .await
    {
        Ok(()) => Ok(HttpResponse::SeeOther()
            .insert_header(("Location", "/rsvp/thanks"))
            .insert_header(("Cache-Control", "no-store"))
            .finish()),
        Err(SaveError::Unavailable) => render(
            form,
            FormErrors {
                general: "We could not save your RSVP. Please try again or contact us.",
                ..Default::default()
            },
            false,
            StatusCode::SERVICE_UNAVAILABLE,
        ),
        Err(SaveError::Uncertain) => render(
            form,
            FormErrors {
                general: "We could not confirm whether your RSVP was received. Please contact us before submitting another response.",
                ..Default::default()
            },
            false,
            StatusCode::SERVICE_UNAVAILABLE,
        ),
    }
}
