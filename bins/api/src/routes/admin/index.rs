use actix_web::{Error, error::ErrorInternalServerError, get, web, web::Html};
use askama::Template;

use crate::{AdminSession, Context};

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
struct Dashboard {
    email: String,
    google_enabled: bool,
    google_linked: bool,
}

#[get("/admin")]
pub async fn get(ctx: web::Data<Context>, admin: AdminSession) -> Result<Html, Error> {
    let google_enabled = ctx.google().is_some();
    let google_linked = if google_enabled {
        ctx.storage()
            .google()
            .get(&admin.email)
            .await
            .map_err(ErrorInternalServerError)?
            .is_some()
    } else {
        false
    };

    Ok(Html::new(
        Dashboard {
            email: admin.email,
            google_enabled,
            google_linked,
        }
        .render()
        .map_err(ErrorInternalServerError)?,
    ))
}
