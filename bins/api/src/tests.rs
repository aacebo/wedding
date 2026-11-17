use crate::{configure_routes, notion::Notion};
use actix_web::{
    App,
    http::{StatusCode, header},
    test, web,
};

#[actix_web::test]
async fn pages_are_public_and_admin_routes_are_gone() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Notion::new(None, None, None, None)))
            .configure(configure_routes),
    )
    .await;
    for path in ["/rsvp", "/rsvp/thanks", "/welcome", "/contact"] {
        let response =
            test::call_service(&app, test::TestRequest::get().uri(path).to_request()).await;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
    }
    for path in ["/admin", "/admin/rsvps", "/guests"] {
        let response =
            test::call_service(&app, test::TestRequest::get().uri(path).to_request()).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
    }
}

#[actix_web::test]
async fn invalid_form_escapes_input_and_missing_notion_is_not_success() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Notion::new(None, None, None, None)))
            .configure(configure_routes),
    )
    .await;
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/rsvp")
            .set_form([
                ("submission_id", "00000000-0000-4000-8000-000000000000"),
                ("invitation_code", "1234"),
                ("first_name", "<script>alert(1)</script>"),
                ("last_name", "Guest"),
                ("email", "invalid"),
                ("attending", ""),
            ])
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = String::from_utf8(test::read_body(response).await.to_vec()).unwrap();
    assert!(!body.contains("<script>alert(1)</script>"));
    assert!(body.contains("&#60;script&#62;"));
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/rsvp")
            .set_form([
                ("submission_id", "00000000-0000-4000-8000-000000000000"),
                ("invitation_code", "1234"),
                ("first_name", "Alex"),
                ("last_name", "Guest"),
                ("email", "guest@example.com"),
                ("attending", "yes"),
                ("guest_count", "1"),
                ("phone", "+15555555555"),
                ("main_course", "Beef"),
            ])
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(!response.headers().contains_key(header::LOCATION));
}

#[actix_web::test]
async fn individual_rsvp_requires_contact_and_meal_details() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Notion::new(None, None, None, None)))
            .configure(configure_routes),
    )
    .await;
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/rsvp")
            .set_form([
                ("submission_id", "00000000-0000-4000-8000-000000000000"),
                ("invitation_code", "1234"),
                ("first_name", "Alex"),
                ("last_name", "Guest"),
                ("email", "guest@example.com"),
            ])
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = String::from_utf8(test::read_body(response).await.to_vec()).unwrap();
    assert!(body.contains("Enter your phone number"));
    assert!(body.contains("Choose your main course"));
}

#[actix_web::test]
async fn submitted_rsvp_is_written_to_notion() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (sender, receiver) = mpsc::channel();
    let server = std::thread::spawn(move || {
        for index in 0..4 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .unwrap();
            let mut data = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let size = stream.read(&mut buffer).unwrap();
                if size == 0 {
                    break;
                }
                data.extend_from_slice(&buffer[..size]);
                if let Some(end) = data.windows(4).position(|part| part == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&data[..end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if data.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            sender.send(String::from_utf8(data).unwrap()).unwrap();
            let body = match index {
                0 => {
                    r#"{"results":[{"properties":{"Guest / Household":{"title":[{"plain_text":"Test household"}]},"Invited Guests":{"number":2}}}]}"#
                }
                1 | 2 => r#"{"results":[]}"#,
                _ => r#"{"id":"test-page"}"#,
            };
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        }
    });
    let notion = web::Data::new(Notion::for_test(format!("http://{address}/v1")));
    let app = test::init_service(App::new().app_data(notion).configure(configure_routes)).await;
    let form = [
        ("submission_id", "00000000-0000-4000-8000-000000000000"),
        ("invitation_code", "1234"),
        ("first_name", "Alex"),
        ("last_name", "Guest"),
        ("email", "guest@example.com"),
        ("phone", "+15555555555"),
        ("accommodation", "Hotel, Florence"),
        ("dietary_requirements", "Vegetarian"),
        ("main_course", "Not selected"),
    ];
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/rsvp")
            .set_form(form)
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let guest_query = receiver.recv().unwrap();
    let query = receiver.recv().unwrap();
    let invitation_query = receiver.recv().unwrap();
    let create = receiver.recv().unwrap();
    server.join().unwrap();
    assert!(guest_query.starts_with("POST /v1/data_sources/test-guests/query"));
    assert!(query.starts_with("POST /v1/data_sources/test-source/query"));
    assert!(invitation_query.starts_with("POST /v1/data_sources/test-source/query"));
    assert!(create.starts_with("POST /v1/pages"));
    let body: serde_json::Value =
        serde_json::from_str(create.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(
        body["properties"]["First Name"]["rich_text"][0]["text"]["content"],
        "Alex"
    );
    assert_eq!(
        body["properties"]["RSVP Status"]["select"]["name"],
        "Confirmed"
    );
    assert_eq!(
        body["properties"]["Dietary Requirements"]["rich_text"][0]["text"]["content"],
        "Vegetarian"
    );
    assert_eq!(
        body["properties"]["Invitation Code"]["rich_text"][0]["text"]["content"],
        "1234"
    );
}
