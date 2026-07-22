use actix_files::Files;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use actix_web::{App, HttpServer, middleware::NormalizePath, web};
use sqlx::postgres::PgPoolOptions;

mod admin_session;
mod config;
mod context;
mod request_context;
mod routes;

pub use admin_session::AdminSession;
pub use config::Config;
pub use context::Context;
pub use request_context::{RequestContext, RequestContextMiddleware};

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

    let ctx = Context::new(
        pool,
        config.admin_allowlist.clone(),
        config.dev_login_enabled,
    );
    // Derives a stable signing key from the configured secret so admin session
    // cookies survive restarts (as long as SESSION_SECRET is stable).
    let session_key = Key::derive_from(config.session_secret.as_bytes());
    let cookie_secure = config.cookie_secure;
    println!("Starting server at http://0.0.0.0:{}", config.port);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(ctx.clone()))
            .wrap(NormalizePath::trim())
            .wrap(RequestContextMiddleware)
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), session_key.clone())
                    .cookie_name("admin_session".to_string())
                    .cookie_secure(cookie_secure)
                    .build(),
            )
            .service(routes::index::get)
            .service(routes::guests::list::get)
            .service(routes::guests::create::post)
            .service(routes::rsvps::create::post)
            .service(routes::web::welcome::get)
            .service(routes::web::welcome::details)
            .service(routes::web::contact::get)
            .service(routes::web::story::get)
            .service(routes::web::schedule::get)
            .service(routes::web::travel::get)
            .service(routes::web::venue::get)
            // Hidden admin area. Handlers taking `AdminSession` 404 for anyone
            // not signed in as an allowlisted user.
            .service(routes::admin::index::get)
            .service(routes::admin::healthz::get)
            .service(routes::admin::dev_login::post)
            .service(routes::admin::logout::post)
            // Served from disk relative to the working directory the server is
            // launched from (repo root /app in Docker — see compose & Dockerfile).
            .service(Files::new("/assets", "bins/api/assets"))
    })
    .bind(("0.0.0.0", config.port))?
    .run()
    .await
}
