use actix_files::Files;
use actix_web::{App, HttpServer, middleware::NormalizePath, web};

mod config;
mod notion;
mod routes;
mod rsvp_validation;

#[cfg(test)]
mod tests;

pub use config::Config;

fn configure_routes(config: &mut web::ServiceConfig) {
    config
        .app_data(web::FormConfig::default().limit(8192).error_handler(|error, _| {
            let status = if matches!(error, actix_web::error::UrlencodedError::Overflow { .. }) {
                actix_web::http::StatusCode::PAYLOAD_TOO_LARGE
            } else {
                actix_web::http::StatusCode::BAD_REQUEST
            };
            actix_web::error::InternalError::from_response(
                error,
                actix_web::HttpResponse::build(status)
                    .insert_header(("Cache-Control", "no-store"))
                    .body("Unable to read RSVP form. Use the RSVP page and keep your submission under 8 KB."),
            ).into()
        }))
        .app_data(web::JsonConfig::default().limit(8192))
        .service(routes::index::get)
        .service(routes::web::welcome::get)
        .service(routes::web::welcome::details)
        .service(routes::web::contact::get)
        .service(routes::web::story::get)
        .service(routes::web::schedule::get)
        .service(routes::web::travel::get)
        .service(routes::web::venue::get)
        .service(routes::web::rsvp::get)
        .service(routes::web::rsvp::post)
        .service(routes::web::rsvp::thanks)
        .service(routes::web::progress::refresh)
        .service(Files::new("/assets", "bins/api/assets"));
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = Config::from_env();
    if config.notion_token.is_none()
        || config.notion_data_source_id.is_none()
        || config.notion_guests_data_source_id.is_none()
        || config.notion_guests_page_id.is_none()
    {
        eprintln!(
            "RSVP submissions are disabled: configure NOTION_TOKEN, NOTION_RSVP_DATA_SOURCE_ID, NOTION_GUESTS_DATA_SOURCE_ID, and NOTION_GUESTS_PAGE_ID"
        );
    }
    let notion = web::Data::new(notion::Notion::new(
        config.notion_token.clone(),
        config.notion_data_source_id.clone(),
        config.notion_guests_data_source_id.clone(),
        config.notion_guests_page_id.clone(),
    ));
    let notion_progress_webhook_secret = web::Data::new(config.notion_progress_webhook_secret);
    println!("Starting server at http://0.0.0.0:{}", config.port);

    HttpServer::new(move || {
        App::new()
            .app_data(notion.clone())
            .app_data(notion_progress_webhook_secret.clone())
            .wrap(NormalizePath::trim())
            .configure(configure_routes)
    })
    .bind(("0.0.0.0", config.port))?
    .run()
    .await
}
