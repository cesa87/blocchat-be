use actix_web::{get, HttpResponse, Responder};
use serde_json::json;

#[get("/health")]
pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(json!({
        "status": "healthy",
        "service": "blocchat-backend",
        "version": env!("CARGO_PKG_VERSION")
    }))
}
