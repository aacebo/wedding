use actix_web::{
    App,
    http::{StatusCode, header},
    test, web,
};
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{
    Context, RequestContextMiddleware,
    admin_auth::{AdminAccess, AdminCredentials},
    configure_routes,
};

const AUTH: &str = "Basic YWRtaW46cGFzc3dvcmQ=";

fn credentials() -> Option<AdminCredentials> {
    AdminCredentials::from_values(Some("admin".into()), Some("password".into()))
}

macro_rules! application {
    ($pool:expr, $credentials:expr) => {
        App::new()
            .app_data(web::Data::new(Context::new($pool)))
            .app_data(web::Data::new(AdminAccess($credentials)))
            .app_data(
                actix_web_httpauth::extractors::basic::Config::default().realm("Wedding admin"),
            )
            .wrap(RequestContextMiddleware)
            .configure(configure_routes)
    };
}

#[actix_web::test]
async fn public_pages_work_without_database_and_private_routes_require_auth() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let app = test::init_service(application!(pool, credentials())).await;
    for path in ["/rsvp", "/rsvp/thanks", "/welcome", "/contact"] {
        let response =
            test::call_service(&app, test::TestRequest::get().uri(path).to_request()).await;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        let body = String::from_utf8(test::read_body(response).await.to_vec()).unwrap();
        assert!(body.contains("href=\"/rsvp\""));
    }
    for (method, path) in [
        (actix_web::http::Method::GET, "/admin/rsvps".to_owned()),
        (actix_web::http::Method::GET, "/guests".to_owned()),
        (actix_web::http::Method::POST, "/guests".to_owned()),
        (
            actix_web::http::Method::POST,
            format!("/guests/{}/rsvps", uuid::Uuid::nil()),
        ),
    ] {
        for auth in [
            None,
            Some("not basic"),
            Some("Basic !!!"),
            Some("Basic YWRtaW46d3Jvbmc="),
        ] {
            let mut request = test::TestRequest::default()
                .method(method.clone())
                .uri(&path);
            if let Some(auth) = auth {
                request = request.insert_header((header::AUTHORIZATION, auth));
            }
            let response = test::call_service(&app, request.to_request()).await;
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
            assert!(response.headers().contains_key(header::WWW_AUTHENTICATE));
            assert_eq!(
                response.headers().get(header::CACHE_CONTROL).unwrap(),
                "no-store"
            );
        }
    }
    let response =
        test::call_service(&app, test::TestRequest::get().uri("/missing").to_request()).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn disabled_admin_rejects_even_valid_credentials() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let app = test::init_service(application!(pool, None)).await;
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/admin/rsvps")
            .insert_header((header::AUTHORIZATION, AUTH))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn invalid_forms_preserve_escaped_values_and_reject_large_payloads() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let app = test::init_service(application!(pool, None)).await;
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/rsvp")
            .set_form([
                ("name", "<script>alert(1)</script>"),
                ("email", "bad-email"),
                ("attending", ""),
                ("party_size", "3"),
            ])
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = String::from_utf8(test::read_body(response).await.to_vec()).unwrap();
    assert!(body.contains("aria-invalid=\"true\""));
    assert!(body.contains("Choose whether you will attend"));
    assert!(body.contains("Enter a valid email address"));
    assert!(!body.contains("<script>alert(1)</script>"));
    assert!(body.contains("&#60;script&#62;"));
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/rsvp")
            .insert_header((header::CONTENT_TYPE, "application/x-www-form-urlencoded"))
            .set_payload(format!("name={}", "a".repeat(8193)))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[sqlx::test(migrations = "../../crates/storage/migrations")]
async fn form_saves_once_and_admin_shows_correct_totals(pool: PgPool) {
    let app = test::init_service(application!(pool.clone(), credentials())).await;
    for (name, email, attending, size) in [
        ("<Guest>", " Guest@Example.com ", "yes", "2"),
        ("Declining", "declining@example.com", "no", "2"),
    ] {
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/rsvp")
                .set_form([
                    ("name", name),
                    ("email", email),
                    ("attending", attending),
                    ("party_size", size),
                ])
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get(header::LOCATION).unwrap(),
            "/rsvp/thanks"
        );
    }
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/rsvp")
            .set_form([
                ("name", "Overwrite"),
                ("email", "GUEST@example.com"),
                ("attending", "no"),
                ("party_size", "1"),
            ])
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body = String::from_utf8(test::read_body(response).await.to_vec()).unwrap();
    assert!(body.contains("original response has not been changed"));
    assert!(body.contains("href=\"/contact\""));
    assert!(!body.contains("&#60;Guest&#62;"));
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/admin/rsvps")
            .insert_header((header::AUTHORIZATION, AUTH))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL).unwrap(),
        "no-store"
    );
    let body = String::from_utf8(test::read_body(response).await.to_vec()).unwrap();
    assert!(body.contains("2 responses &middot; 2 attending people"));
    assert!(body.contains("&#60;Guest&#62;"));
    assert!(body.contains("guest@example.com"));
    assert!(body.contains("Declining"));
    let responses = storage::RsvpStorage::new(&pool).responses().await.unwrap();
    assert_eq!(responses.iter().map(|row| row.party_size).sum::<i16>(), 2);
    assert!(!responses.iter().any(|row| row.name == "Overwrite"));
}

#[sqlx::test(migrations = "../../crates/storage/migrations")]
async fn authenticated_legacy_api_uses_defaults_and_prevents_duplicates(pool: PgPool) {
    let app = test::init_service(application!(pool.clone(), credentials())).await;
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/admin/rsvps")
            .insert_header((header::AUTHORIZATION, AUTH))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        String::from_utf8(test::read_body(response).await.to_vec())
            .unwrap()
            .contains("No RSVP responses")
    );
    for (index, attending, expected_size) in [(1, true, 1), (2, false, 0)] {
        let guest = storage::types::Guest::new(
            format!("Guest {index}"),
            format!("api{index}@example.com"),
            false,
        );
        storage::GuestStorage::new(&pool)
            .create(&guest)
            .await
            .unwrap();
        let path = format!("/guests/{}/rsvps", guest.id);
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&path)
                .insert_header((header::AUTHORIZATION, AUTH))
                .insert_header((header::CONTENT_TYPE, "application/json"))
                .set_payload(format!(r#"{{"attending":{attending}}}"#))
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = String::from_utf8(test::read_body(response).await.to_vec()).unwrap();
        assert!(body.contains(&format!("\"party_size\":{expected_size}")));
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&path)
                .insert_header((header::AUTHORIZATION, AUTH))
                .insert_header((header::CONTENT_TYPE, "application/json"))
                .set_payload(r#"{"attending":true,"party_size":2}"#)
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&path)
                .insert_header((header::AUTHORIZATION, AUTH))
                .insert_header((header::CONTENT_TYPE, "application/json"))
                .set_payload(r#"{"attending":true,"party_size":3}"#)
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/guests/{}/rsvps", uuid::Uuid::nil()))
            .insert_header((header::AUTHORIZATION, AUTH))
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .set_payload(r#"{"attending":true}"#)
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn storage_failure_is_not_success() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    pool.close().await;
    let app = test::init_service(application!(pool, credentials())).await;
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/rsvp")
            .set_form([
                ("name", "Guest"),
                ("email", "guest@example.com"),
                ("attending", "yes"),
                ("party_size", "1"),
            ])
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(!response.headers().contains_key(header::LOCATION));
    assert!(
        String::from_utf8(test::read_body(response).await.to_vec())
            .unwrap()
            .contains("could not save")
    );
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/admin/rsvps")
            .insert_header((header::AUTHORIZATION, AUTH))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL).unwrap(),
        "no-store"
    );
}

#[sqlx::test(migrations = "../../crates/storage/migrations")]
async fn guest_api_normalizes_identity_and_requires_valid_fields(pool: PgPool) {
    let app = test::init_service(application!(pool, credentials())).await;
    for (body, status) in [
        (
            r#"{"name":" Guest ","email":" GUEST@example.com ","plus_one":true}"#,
            StatusCode::CREATED,
        ),
        (
            r#"{"name":"Duplicate","email":"guest@example.com","plus_one":false}"#,
            StatusCode::CONFLICT,
        ),
        (
            r#"{"name":" ","email":"valid@example.com","plus_one":false}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            r#"{"name":"Guest","email":"invalid","plus_one":false}"#,
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/guests")
                .insert_header((header::AUTHORIZATION, AUTH))
                .insert_header((header::CONTENT_TYPE, "application/json"))
                .set_payload(body)
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), status);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
    }
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/guests")
            .insert_header((header::AUTHORIZATION, AUTH))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = String::from_utf8(test::read_body(response).await.to_vec()).unwrap();
    assert!(body.contains("\"name\":\"Guest\""));
    assert!(body.contains("\"email\":\"guest@example.com\""));
    assert!(!body.contains("Duplicate"));
}
