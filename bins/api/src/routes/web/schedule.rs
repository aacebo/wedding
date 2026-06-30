use actix_web::{Error, error::ErrorInternalServerError, get, web::Html};
use askama::Template;

#[derive(Template)]
#[template(path = "schedule.html")]
struct SchedulePage;

#[get("/schedule")]
pub async fn get() -> Result<Html, Error> {
    Ok(Html::new(
        SchedulePage.render().map_err(ErrorInternalServerError)?,
    ))
}
