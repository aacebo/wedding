use actix_web::{Error, error::ErrorInternalServerError, get, web::Html};
use askama::Template;

use super::PageMetadata;

#[derive(Template)]
#[template(path = "travel.html")]
struct TravelPage {
    meta: PageMetadata,
}

#[get("/travel")]
pub async fn get() -> Result<Html, Error> {
    let page = TravelPage {
        meta: PageMetadata::new(
            "Travel & Stay — Nancy & Alexander",
            "Travel and lodging recommendations for Nancy and Alexander's wedding near Villa di Striano in Tuscany.",
            "https://baicebo.com/travel",
        ),
    };

    Ok(Html::new(page.render().map_err(ErrorInternalServerError)?))
}
