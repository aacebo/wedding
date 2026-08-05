use actix_web::{Error, error::ErrorInternalServerError, get, web::Html};
use askama::Template;

use super::PageMetadata;

#[derive(Template)]
#[template(path = "welcome.html")]
struct WelcomePage {
    meta: PageMetadata,
}

#[derive(Template)]
#[template(path = "welcome_details.html")]
struct WelcomeDetails;

#[get("/welcome")]
pub async fn get() -> Result<Html, Error> {
    let page = WelcomePage {
        meta: PageMetadata::new(
            "Nancy & Alexander — The Wedding Edition",
            "Join Nancy and Alexander for their wedding celebration on June 19, 2027, at Villa di Striano in Tuscany.",
            "https://baicebo.com/welcome",
        ),
    };

    Ok(Html::new(page.render().map_err(ErrorInternalServerError)?))
}

#[get("/welcome/details")]
pub async fn details() -> Result<Html, Error> {
    Ok(Html::new(
        WelcomeDetails.render().map_err(ErrorInternalServerError)?,
    ))
}
