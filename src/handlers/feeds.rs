use actix_web::{delete, get, post, put, web, HttpResponse, Responder, Scope};
use sqlx::PgPool;

use crate::models::feed::{
    CreateFeedSubscriptionRequest, FeedSubscriptionResponse, UpdateFeedSubscriptionRequest,
};
use crate::services::feed_service;

// ── Subscriptions ──

#[post("/conversations/{conversation_id}/subscriptions")]
async fn create_subscription(
    pool: web::Data<PgPool>,
    conversation_id: web::Path<String>,
    req: web::Json<CreateFeedSubscriptionRequest>,
) -> impl Responder {
    match feed_service::create_subscription(&pool, &conversation_id, req.into_inner()).await {
        Ok(sub) => HttpResponse::Created().json(FeedSubscriptionResponse::from(sub)),
        Err(e) => {
            log::error!("Failed to create feed subscription: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to create feed subscription"
            }))
        }
    }
}

#[get("/conversations/{conversation_id}/subscriptions")]
async fn get_subscriptions(
    pool: web::Data<PgPool>,
    conversation_id: web::Path<String>,
) -> impl Responder {
    match feed_service::get_subscriptions_for_conversation(&pool, &conversation_id).await {
        Ok(subs) => {
            let results: Vec<FeedSubscriptionResponse> =
                subs.into_iter().map(FeedSubscriptionResponse::from).collect();
            HttpResponse::Ok().json(results)
        }
        Err(e) => {
            log::error!("Failed to get feed subscriptions: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to get feed subscriptions"
            }))
        }
    }
}

#[put("/subscriptions/{subscription_id}")]
async fn update_subscription(
    pool: web::Data<PgPool>,
    subscription_id: web::Path<String>,
    req: web::Json<UpdateFeedSubscriptionRequest>,
) -> impl Responder {
    let id = match uuid::Uuid::parse_str(&subscription_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid subscription ID"
            }))
        }
    };

    match feed_service::update_subscription(&pool, &id, req.into_inner()).await {
        Ok(sub) => HttpResponse::Ok().json(FeedSubscriptionResponse::from(sub)),
        Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Subscription not found"
        })),
        Err(e) => {
            log::error!("Failed to update feed subscription: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to update feed subscription"
            }))
        }
    }
}

#[delete("/subscriptions/{subscription_id}")]
async fn delete_subscription(
    pool: web::Data<PgPool>,
    subscription_id: web::Path<String>,
) -> impl Responder {
    let id = match uuid::Uuid::parse_str(&subscription_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid subscription ID"
            }))
        }
    };

    match feed_service::delete_subscription(&pool, &id).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            log::error!("Failed to delete feed subscription: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to delete feed subscription"
            }))
        }
    }
}

// ── State ──

#[get("/conversations/{conversation_id}/state")]
async fn get_state(
    pool: web::Data<PgPool>,
    conversation_id: web::Path<String>,
) -> impl Responder {
    match feed_service::get_feed_state(&pool, &conversation_id).await {
        Ok(state) => HttpResponse::Ok().json(state),
        Err(e) => {
            log::error!("Failed to get feed state: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to get feed state"
            }))
        }
    }
}

// ── Events ──

#[post("/events/{event_id}/seen")]
async fn mark_event_seen(
    pool: web::Data<PgPool>,
    event_id: web::Path<String>,
) -> impl Responder {
    let id = match uuid::Uuid::parse_str(&event_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid event ID"
            }))
        }
    };

    match feed_service::mark_event_seen(&pool, &id).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Err(e) => {
            log::error!("Failed to mark feed event seen: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to mark event seen"
            }))
        }
    }
}

pub fn configure() -> Scope {
    web::scope("/feeds")
        .service(create_subscription)
        .service(get_subscriptions)
        .service(update_subscription)
        .service(delete_subscription)
        .service(get_state)
        .service(mark_event_seen)
}
