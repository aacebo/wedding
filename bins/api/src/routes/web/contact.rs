use actix_web::{Error, error::ErrorInternalServerError, get, web::Html};
use askama::Template;

#[derive(Template)]
#[template(path = "contact.html")]
struct ContactPage;

#[get("/contact")]
pub async fn get() -> Result<Html, Error> {
    Ok(Html::new(
        ContactPage.render().map_err(ErrorInternalServerError)?,
    ))
}
