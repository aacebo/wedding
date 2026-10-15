use actix_files::Files;
use actix_web::{
    App, HttpServer,
    middleware::{DefaultHeaders, NormalizePath},
    web,
};
use actix_web_httpauth::middleware::HttpAuthentication;
use sqlx::postgres::PgPoolOptions;

mod admin_auth;
mod config;
mod context;
mod request_context;
mod routes;
mod rsvp_validation;

#[cfg(test)]
mod tests;

pub use config::Config;
pub use context::Context;
pub use request_context::{RequestContext, RequestContextMiddleware};

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
        .service(Files::new("/assets", "bins/api/assets"))
        .service(
            web::scope("")
                .guard(actix_web::guard::fn_guard(|context| {
                    let path = context.head().uri.path();
                    path == "/guests" || path.starts_with("/guests/") || path.starts_with("/admin/")
                }))
                .wrap(HttpAuthentication::basic(admin_auth::validate))
                .wrap(DefaultHeaders::new()
                    .add(("Cache-Control", "no-store"))
                    .add(("X-Robots-Tag", "noindex, nofollow")))
                .service(routes::guests::list::get)
                .service(routes::guests::create::post)
                .service(routes::rsvps::create::post)
                .service(routes::admin::rsvps),
        );
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = Config::from_env();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("Failed to create pool");

    sqlx::migrate!("../../crates/storage/migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let ctx = Context::new(pool);
    let admin_access = web::Data::new(admin_auth::AdminAccess(config.admin_credentials));
    println!("Starting server at http://0.0.0.0:{}", config.port);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(ctx.clone()))
            .app_data(admin_access.clone())
            .app_data(
                actix_web_httpauth::extractors::basic::Config::default().realm("Wedding admin"),
            )
            .wrap(NormalizePath::trim())
            .wrap(RequestContextMiddleware)
            .configure(configure_routes)
    })
    .bind(("0.0.0.0", config.port))?
    .run()
    .await
}
