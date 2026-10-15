use actix_web::{Error, HttpResponse, error::ErrorInternalServerError, get};
use askama::Template;
use storage::RsvpResponse;

use super::web::PageMetadata;
use crate::RequestContext;

#[derive(Template)]
#[template(path = "admin_rsvps.html")]
struct AdminRsvpsPage {
    meta: PageMetadata,
    responses: Vec<RsvpResponse>,
    attending_people: i64,
}

#[get("/admin/rsvps")]
pub async fn rsvps(ctx: RequestContext) -> Result<HttpResponse, Error> {
    let responses = ctx.storage().rsvps().responses().await.map_err(|error| {
        eprintln!(
            "Admin RSVP list storage error [{}]: {error}",
            ctx.request_id()
        );
        ErrorInternalServerError("Unable to load RSVP responses. Please try again.")
    })?;
    let attending_people = responses
        .iter()
        .map(|response| i64::from(response.party_size))
        .sum();
    let page = AdminRsvpsPage {
        meta: PageMetadata::new(
            "RSVP responses - Nancy & Alexander",
            "Private wedding RSVP response list.",
            "https://baicebo.com/admin/rsvps",
        ),
        responses,
        attending_people,
    };
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(page.render().map_err(ErrorInternalServerError)?))
}
