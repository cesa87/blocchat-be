use actix_web::{delete, get, post, web, HttpResponse, Responder, Scope};
use crate::{
    db::DbPool,
    models::{CreateTokenGateRequest, VerifyTokenGateRequest},
    services::token_gate_service,
};

pub fn configure() -> Scope {
    web::scope("/token-gates")
        .service(create_or_update_gates)
        .service(get_gates)
        .service(delete_gates)
        .service(verify_gates)
}

#[post("/conversations/{conversation_id}")]
async fn create_or_update_gates(
    pool: web::Data<DbPool>,
    conversation_id: web::Path<String>,
    req: web::Json<CreateTokenGateRequest>,
) -> impl Responder {
    match token_gate_service::create_or_update_token_gates(&pool, &conversation_id, req.into_inner()).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Token gates created successfully"
        })),
        Err(e) => {
            log::error!("Failed to create token gates: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to create token gates"
            }))
        }
    }
}

#[get("/conversations/{conversation_id}")]
async fn get_gates(
    pool: web::Data<DbPool>,
    conversation_id: web::Path<String>,
) -> impl Responder {
    match token_gate_service::get_token_gates(&pool, &conversation_id).await {
        Ok(Some(gates)) => HttpResponse::Ok().json(gates),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "No token gates found for this conversation"
        })),
        Err(e) => {
            log::error!("Failed to get token gates: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to get token gates"
            }))
        }
    }
}

#[delete("/conversations/{conversation_id}")]
async fn delete_gates(
    pool: web::Data<DbPool>,
    conversation_id: web::Path<String>,
) -> impl Responder {
    match token_gate_service::delete_token_gates(&pool, &conversation_id).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Token gates deleted successfully"
        })),
        Err(e) => {
            log::error!("Failed to delete token gates: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to delete token gates"
            }))
        }
    }
}

#[post("/verify")]
async fn verify_gates(
    pool: web::Data<DbPool>,
    req: web::Json<VerifyTokenGateRequest>,
) -> impl Responder {
    match token_gate_service::verify_token_gates(&pool, req.into_inner()).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            log::error!("Failed to verify token gates: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to verify token gates: {}", e)
            }))
        }
    }
}
