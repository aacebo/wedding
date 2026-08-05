use actix_web::{Error, error::ErrorInternalServerError, get, web::Html};
use askama::Template;

use super::PageMetadata;

#[derive(Template)]
#[template(path = "contact.html")]
struct ContactPage {
    meta: PageMetadata,
}

#[get("/contact")]
pub async fn get() -> Result<Html, Error> {
    let page = ContactPage {
        meta: PageMetadata::new(
            "Contact — Nancy & Alexander",
            "Contact Nancy and Alexander with questions about their wedding celebration in Tuscany.",
            "https://baicebo.com/contact",
        ),
    };

    Ok(Html::new(page.render().map_err(ErrorInternalServerError)?))
}
