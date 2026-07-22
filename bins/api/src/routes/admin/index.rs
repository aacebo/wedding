use actix_web::{Error, error::ErrorInternalServerError, get, web::Html};
use askama::Template;

use crate::AdminSession;

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
struct Dashboard {
    email: String,
}

#[get("/admin")]
pub async fn get(admin: AdminSession) -> Result<Html, Error> {
    Ok(Html::new(
        Dashboard { email: admin.email }
            .render()
            .map_err(ErrorInternalServerError)?,
    ))
}
