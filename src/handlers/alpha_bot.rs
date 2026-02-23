use actix_web::{delete, get, post, put, web, HttpResponse, Responder, Scope};
use sqlx::PgPool;
use std::collections::HashMap;

use crate::models::alpha_bot::{
    AlphaBotConfigResponse, AlphaBotAlertResponse,
    CreateAlphaBotConfigRequest, UpdateAlphaBotConfigRequest,
};
use crate::services::alpha_bot_service;

#[post("/conversations/{conversation_id}/config")]
async fn create_config(
    pool: web::Data<PgPool>,
    conversation_id: web::Path<String>,
    req: web::Json<CreateAlphaBotConfigRequest>,
) -> impl Responder {
    match alpha_bot_service::create_config(&pool, &conversation_id, req.into_inner()).await {
        Ok(config) => HttpResponse::Created().json(AlphaBotConfigResponse::from(config)),
        Err(e) => {
            log::error!("Failed to create alpha bot config: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to create bot config"
            }))
        }
    }
}

#[get("/conversations/{conversation_id}/config")]
async fn get_configs(
    pool: web::Data<PgPool>,
    conversation_id: web::Path<String>,
) -> impl Responder {
    match alpha_bot_service::get_configs_for_conversation(&pool, &conversation_id).await {
        Ok(configs) => {
            let results: Vec<AlphaBotConfigResponse> =
                configs.into_iter().map(|c| c.into()).collect();
            HttpResponse::Ok().json(results)
        }
        Err(e) => {
            log::error!("Failed to get alpha bot configs: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to get bot configs"
            }))
        }
    }
}

#[put("/config/{config_id}")]
async fn update_config(
    pool: web::Data<PgPool>,
    config_id: web::Path<String>,
    req: web::Json<UpdateAlphaBotConfigRequest>,
) -> impl Responder {
    let id = match uuid::Uuid::parse_str(&config_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid config ID"
            }))
        }
    };

    match alpha_bot_service::update_config(&pool, &id, req.into_inner()).await {
        Ok(config) => HttpResponse::Ok().json(AlphaBotConfigResponse::from(config)),
        Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Config not found"
        })),
        Err(e) => {
            log::error!("Failed to update alpha bot config: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to update bot config"
            }))
        }
    }
}

#[delete("/config/{config_id}")]
async fn delete_config(
    pool: web::Data<PgPool>,
    config_id: web::Path<String>,
) -> impl Responder {
    let id = match uuid::Uuid::parse_str(&config_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid config ID"
            }))
        }
    };

    match alpha_bot_service::delete_config(&pool, &id).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            log::error!("Failed to delete alpha bot config: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to delete bot config"
            }))
        }
    }
}

#[get("/conversations/{conversation_id}/alerts")]
async fn get_alerts(
    pool: web::Data<PgPool>,
    conversation_id: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let since = query
        .get("since")
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    let limit = query
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(50)
        .min(200);

    match alpha_bot_service::get_alerts_for_conversation(&pool, &conversation_id, since, limit)
        .await
    {
        Ok(alerts) => {
            let results: Vec<AlphaBotAlertResponse> =
                alerts.into_iter().map(|a| a.into()).collect();
            HttpResponse::Ok().json(results)
        }
        Err(e) => {
            log::error!("Failed to get alpha bot alerts: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to get alerts"
            }))
        }
    }
}

pub fn configure() -> Scope {
    web::scope("/alpha-bot")
        .service(create_config)
        .service(get_configs)
        .service(update_config)
        .service(delete_config)
        .service(get_alerts)
}
