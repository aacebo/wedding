use actix_web::{Error, error::ErrorInternalServerError, get, web::Html};
use askama::Template;

#[derive(Template)]
#[template(path = "venue.html")]
struct VenuePage;

#[get("/venue")]
pub async fn get() -> Result<Html, Error> {
    Ok(Html::new(
        VenuePage.render().map_err(ErrorInternalServerError)?,
    ))
}
