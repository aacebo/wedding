use actix_web::{HttpResponse, get};

use crate::AdminSession;

#[derive(serde::Serialize)]
struct Health {
    status: &'static str,
    email: String,
}

/// Authenticated smoke-test endpoint. Gated by `AdminSession`, so it doubles as
/// a quick check that the session/allowlist wiring works.
#[get("/admin/healthz")]
pub async fn get(admin: AdminSession) -> HttpResponse {
    HttpResponse::Ok().json(Health {
        status: "ok",
        email: admin.email,
    })
}
