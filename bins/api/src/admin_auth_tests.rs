//! Auth-coverage test: every `/admin/*` handler is gated by the `AdminSession`
//! extractor, which returns **404** (not 401/403) for anyone without a valid,
//! allowlisted session — keeping the admin surface hidden. This verifies the
//! guard end-to-end for a representative route from each admin module without
//! needing a live database (the extractor rejects before any query runs).

use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::{App, cookie::Key, test, web};
use sqlx::postgres::PgPoolOptions;

use admin::Allowlist;

use crate::{Context, routes};

/// A Context backed by a *lazy* pool that never actually connects — safe because
/// the guard short-circuits before touching the database.
fn test_context() -> Context {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://user:pass@localhost/db")
        .expect("lazy pool");
    Context::new(
        pool,
        Allowlist::from_csv("admin@example.com"),
        false,
        None,
        None,
        25,
    )
}

#[actix_web::test]
async fn admin_routes_are_hidden_without_a_session() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(test_context()))
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), Key::derive_from(&[0u8; 64]))
                    .cookie_secure(false)
                    .build(),
            )
            .service(routes::admin::index::get)
            .service(routes::admin::inbox::get)
            .service(routes::admin::todos::get)
            .service(routes::admin::timeline::get)
            .service(routes::admin::notifications::panel)
            .service(routes::admin::sync_status::get),
    )
    .await;

    for path in [
        "/admin",
        "/admin/inbox",
        "/admin/todos",
        "/admin/timeline",
        "/admin/notifications",
        "/admin/sync/status",
    ] {
        let req = test::TestRequest::get().uri(path).to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status().as_u16(),
            404,
            "unauthenticated {path} should 404 to stay hidden"
        );
    }
}
