use actix_web::{Error, error::ErrorInternalServerError, get, web::Html};
use askama::Template;

#[derive(Template)]
#[template(path = "story.html")]
struct StoryPage;

#[get("/story")]
pub async fn get() -> Result<Html, Error> {
    Ok(Html::new(
        StoryPage.render().map_err(ErrorInternalServerError)?,
    ))
}
