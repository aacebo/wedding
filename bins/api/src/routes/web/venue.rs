use actix_web::{Error, error::ErrorInternalServerError, get, web::Html};
use askama::Template;

use super::PageMetadata;

#[derive(Template)]
#[template(path = "venue.html")]
struct VenuePage {
    meta: PageMetadata,
}

#[get("/venue")]
pub async fn get() -> Result<Html, Error> {
    let page = VenuePage {
        meta: PageMetadata::new(
            "Villa di Striano — Nancy & Alexander",
            "Wedding details for Villa di Striano in Borgo San Lorenzo, Tuscany.",
            "https://baicebo.com/venue",
        ),
    };

    Ok(Html::new(page.render().map_err(ErrorInternalServerError)?))
}
