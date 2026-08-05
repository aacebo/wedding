use actix_web::{Error, error::ErrorInternalServerError, get, web::Html};
use askama::Template;

use super::PageMetadata;

#[derive(Template)]
#[template(path = "schedule.html")]
struct SchedulePage {
    meta: PageMetadata,
}

#[get("/schedule")]
pub async fn get() -> Result<Html, Error> {
    let page = SchedulePage {
        meta: PageMetadata::new(
            "Schedule — Nancy & Alexander",
            "Wedding-day schedule for Nancy and Alexander at Villa di Striano on June 19, 2027.",
            "https://baicebo.com/schedule",
        ),
    };

    Ok(Html::new(page.render().map_err(ErrorInternalServerError)?))
}
