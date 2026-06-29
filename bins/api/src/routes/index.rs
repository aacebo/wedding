use actix_web::{HttpResponse, get};
use askama::Template;
use serde::Serialize;

use crate::RequestContext;

#[derive(Serialize)]
struct HealthResponse {
    start_time: String,
}

#[derive(Template)]
#[template(path = "index/page.html")]
struct IndexPage;

#[get("/")]
pub async fn get(ctx: RequestContext) -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse {
        start_time: ctx.start_time().to_rfc3339(),
    })
}

#[get("/ui")]
pub async fn ui() -> Result<actix_web::web::Html, actix_web::Error> {
    Ok(actix_web::web::Html::new(
        IndexPage
            .render()
            .map_err(actix_web::error::ErrorInternalServerError)?,
    ))
}
