use actix_web::{Error, error::ErrorInternalServerError, get, web::Html};
use askama::Template;

#[derive(Template)]
#[template(path = "travel.html")]
struct TravelPage;

#[get("/travel")]
pub async fn get() -> Result<Html, Error> {
    Ok(Html::new(
        TravelPage.render().map_err(ErrorInternalServerError)?,
    ))
}
