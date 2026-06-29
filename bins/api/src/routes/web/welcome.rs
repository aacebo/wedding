use actix_web::{Error, error::ErrorInternalServerError, get, web::Html};
use askama::Template;

#[derive(Template)]
#[template(path = "welcome.html")]
struct WelcomePage;

#[derive(Template)]
#[template(path = "welcome_details.html")]
struct WelcomeDetails;

#[get("/welcome")]
pub async fn get() -> Result<Html, Error> {
    Ok(Html::new(
        WelcomePage.render().map_err(ErrorInternalServerError)?,
    ))
}

#[get("/welcome/details")]
pub async fn details() -> Result<Html, Error> {
    Ok(Html::new(
        WelcomeDetails.render().map_err(ErrorInternalServerError)?,
    ))
}
