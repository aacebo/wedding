use actix_web::{HttpResponse, get, http::header};

#[get("/")]
pub async fn get() -> HttpResponse {
    HttpResponse::Found()
        .insert_header((header::LOCATION, "/welcome"))
        .finish()
}
