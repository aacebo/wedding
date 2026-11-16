use actix_web::{
    Error, HttpResponse, error::ErrorInternalServerError, get, http::StatusCode, post, web,
};
use askama::Template;
use storage::SubmitRsvpError;

use super::PageMetadata;
use crate::RequestContext;
use crate::rsvp_validation::{valid_email, valid_name};

#[derive(Default, serde::Deserialize)]
#[serde(default)]
pub(crate) struct RsvpForm {
    first_name: String,
    last_name: String,
    email: String,
    attending: String,
    accommodation_name: String,
    accommodation_address: String,
    phone: String,
    dietary_requirements: String,
    meal_preference: String,
}

#[derive(Default)]
struct FormErrors {
    first_name: &'static str,
    last_name: &'static str,
    email: &'static str,
    attending: &'static str,
    accommodation: &'static str,
    phone: &'static str,
    dietary: &'static str,
    meal: &'static str,
    general: &'static str,
}

impl FormErrors {
    fn is_empty(&self) -> bool {
        self.first_name.is_empty()
            && self.last_name.is_empty()
            && self.email.is_empty()
            && self.attending.is_empty()
            && self.accommodation.is_empty()
            && self.phone.is_empty()
            && self.dietary.is_empty()
            && self.meal.is_empty()
            && self.general.is_empty()
    }
}

fn valid_text(value: &str, max_chars: usize) -> bool {
    value.chars().count() <= max_chars && !value.chars().any(char::is_control)
}

impl RsvpForm {
    fn validate(&mut self) -> Result<(bool, Option<String>), FormErrors> {
        self.first_name = self.first_name.trim().to_owned();
        self.last_name = self.last_name.trim().to_owned();
        self.email = self.email.trim().to_lowercase();
        self.accommodation_name = self.accommodation_name.trim().to_owned();
        self.accommodation_address = self.accommodation_address.trim().to_owned();
        self.phone = self.phone.trim().to_owned();
        self.dietary_requirements = self.dietary_requirements.trim().to_owned();

        let mut errors = FormErrors::default();
        if !valid_name(&self.first_name) {
            errors.first_name = "Enter your first name (up to 200 characters).";
        }
        if !valid_name(&self.last_name) {
            errors.last_name = "Enter your last name (up to 200 characters).";
        }
        let full_name = format!("{} {}", self.first_name, self.last_name);
        if full_name.chars().count() > 200 {
            errors.last_name = "Your full name must be 200 characters or fewer.";
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

        if attending {
            if !valid_text(&self.accommodation_name, 200)
                || self.accommodation_name.is_empty()
                || !valid_text(&self.accommodation_address, 500)
                || self.accommodation_address.is_empty()
            {
                errors.accommodation = "Enter the name and address of your accommodation.";
            }
            let digits = self.phone.chars().filter(char::is_ascii_digit).count();
            let phone_chars_valid = self
                .phone
                .chars()
                .all(|ch| ch.is_ascii_digit() || matches!(ch, '+' | '-' | '.' | '(' | ')' | ' '));
            if !phone_chars_valid || !(7..=30).contains(&self.phone.chars().count()) || digits < 7 {
                errors.phone = "Enter a phone number with country code.";
            }
        } else if !valid_text(&self.accommodation_name, 200)
            || !valid_text(&self.accommodation_address, 500)
        {
            errors.accommodation = "Use plain text for accommodation details.";
        } else if !valid_text(&self.phone, 30) {
            errors.phone = "Use a phone number up to 30 characters.";
        }

        if !valid_text(&self.dietary_requirements, 1000) {
            errors.dietary = "Use plain text (up to 1,000 characters).";
        }

        let meal_preference = if attending {
            match self.meal_preference.as_str() {
                "beef" => Some("beef".to_owned()),
                "sea_bass" => Some("sea_bass".to_owned()),
                _ => {
                    errors.meal = "Choose beef or sea bass.";
                    None
                }
            }
        } else {
            None
        };

        if errors.is_empty() {
            Ok((attending, meal_preference))
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
        RsvpForm::default(),
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
    let (attending, meal_preference) = match form.validate() {
        Ok(values) => values,
        Err(errors) => return render(form, errors, false, StatusCode::UNPROCESSABLE_ENTITY),
    };

    let first_name = form.first_name.clone();
    let last_name = form.last_name.clone();
    let email = form.email.clone();
    let accommodation_name = if attending { Some(form.accommodation_name.clone()) } else { None };
    let accommodation_address = if attending { Some(form.accommodation_address.clone()) } else { None };
    let phone = if attending { Some(form.phone.clone()) } else { None };
    let dietary_requirements = if form.dietary_requirements.is_empty() {
        None
    } else {
        Some(form.dietary_requirements.clone())
    };

    match ctx
        .storage()
        .rsvps()
        .submit_individual(
            &first_name,
            &last_name,
            &email,
            attending,
            accommodation_name.as_deref(),
            accommodation_address.as_deref(),
            phone.as_deref(),
            dietary_requirements.as_deref(),
            meal_preference.as_deref(),
        )
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
