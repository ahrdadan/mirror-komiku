use actix_web::{get, HttpResponse};
use serde::Serialize;

#[derive(Serialize)]
struct HealthPayload {
    status: &'static str,
}

#[get("/health")]
pub async fn health() -> HttpResponse {
    HttpResponse::Ok().json(HealthPayload { status: "ok" })
}
