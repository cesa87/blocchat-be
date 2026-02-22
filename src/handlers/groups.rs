use actix_web::{delete, get, post, put, web, HttpResponse, Responder, Scope};
use sqlx::PgPool;

use crate::models::group::{CreatePublicGroupRequest, UpdatePublicGroupRequest, PublicGroupResponse};
use crate::services::group_service;

#[post("")]
async fn register_group(
    pool: web::Data<PgPool>,
    req: web::Json<CreatePublicGroupRequest>,
) -> impl Responder {
    match group_service::register_group(&pool, req.into_inner()).await {
        Ok(group) => HttpResponse::Created().json(PublicGroupResponse::from(group)),
        Err(e) => {
            log::error!("Failed to register group: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to register group"
            }))
        }
    }
}

#[get("/search")]
async fn search_groups(
    pool: web::Data<PgPool>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let q = query.get("q").map(|s| s.as_str()).unwrap_or("");
    if q.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Search query required"
        }));
    }

    let limit = query
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(20)
        .min(50);

    match group_service::search_groups(&pool, q, limit).await {
        Ok(groups) => {
            let results: Vec<PublicGroupResponse> = groups.into_iter().map(|g| g.into()).collect();
            HttpResponse::Ok().json(results)
        }
        Err(e) => {
            log::error!("Group search failed: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Search failed"
            }))
        }
    }
}

#[get("/{conversation_id}")]
async fn get_group(
    pool: web::Data<PgPool>,
    conversation_id: web::Path<String>,
) -> impl Responder {
    match group_service::get_group(&pool, &conversation_id).await {
        Ok(group) => HttpResponse::Ok().json(PublicGroupResponse::from(group)),
        Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Group not found"
        })),
        Err(e) => {
            log::error!("Failed to get group: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to get group"
            }))
        }
    }
}

#[put("/{conversation_id}")]
async fn update_group(
    pool: web::Data<PgPool>,
    conversation_id: web::Path<String>,
    req: web::Json<UpdatePublicGroupRequest>,
) -> impl Responder {
    match group_service::update_group(&pool, &conversation_id, req.into_inner()).await {
        Ok(group) => HttpResponse::Ok().json(PublicGroupResponse::from(group)),
        Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Group not found"
        })),
        Err(e) => {
            log::error!("Failed to update group: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to update group"
            }))
        }
    }
}

#[delete("/{conversation_id}")]
async fn delete_group(
    pool: web::Data<PgPool>,
    conversation_id: web::Path<String>,
) -> impl Responder {
    match group_service::delete_group(&pool, &conversation_id).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            log::error!("Failed to delete group: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to delete group"
            }))
        }
    }
}

pub fn configure() -> Scope {
    web::scope("/groups")
        .service(search_groups)  // Must come before /{conversation_id}
        .service(register_group)
        .service(get_group)
        .service(update_group)
        .service(delete_group)
}
