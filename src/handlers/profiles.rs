use actix_web::{get, post, put, web, HttpResponse, Responder, Scope};
use sqlx::PgPool;

use crate::models::{ClaimUsernameRequest, UpdateProfileRequest, ProfileResponse};
use crate::services::profile_service;

pub fn configure() -> Scope {
    web::scope("/profiles")
        .service(get_or_create)
        .service(search)  // Must come before /{wallet_address}
        .service(check_username)
        .service(get_by_username)
        .service(get_by_inbox_id)
        .service(claim_username)
        .service(update_profile)
        .service(get_by_wallet)  // Must come last since it catches any path
}

/// Get or create profile (called on app load)
#[post("/init")]
async fn get_or_create(
    pool: web::Data<PgPool>,
    req: web::Json<serde_json::Value>,
) -> impl Responder {
    let wallet_address = match req.get("wallet_address").and_then(|v| v.as_str()) {
        Some(w) => w,
        None => return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "wallet_address is required"
        })),
    };
    
    let inbox_id = match req.get("inbox_id").and_then(|v| v.as_str()) {
        Some(i) => i,
        None => return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "inbox_id is required"
        })),
    };
    
    match profile_service::get_or_create_profile(&pool, wallet_address, inbox_id).await {
        Ok(profile) => HttpResponse::Ok().json(ProfileResponse::from(profile)),
        Err(e) => {
            log::error!("Failed to get/create profile: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to initialize profile"
            }))
        }
    }
}

/// Get profile by wallet address
#[get("/{wallet_address}")]
async fn get_by_wallet(
    pool: web::Data<PgPool>,
    wallet: web::Path<String>,
) -> impl Responder {
    match profile_service::get_profile_by_wallet(&pool, &wallet).await {
        Ok(profile) => HttpResponse::Ok().json(ProfileResponse::from(profile)),
        Err(_) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Profile not found"
        })),
    }
}

/// Get profile by username
#[get("/username/{username}")]
async fn get_by_username(
    pool: web::Data<PgPool>,
    username: web::Path<String>,
) -> impl Responder {
    match profile_service::get_profile_by_username(&pool, &username).await {
        Ok(profile) => HttpResponse::Ok().json(ProfileResponse::from(profile)),
        Err(_) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "User not found"
        })),
    }
}

/// Get profile by inbox_id
#[get("/inbox/{inbox_id}")]
async fn get_by_inbox_id(
    pool: web::Data<PgPool>,
    inbox_id: web::Path<String>,
) -> impl Responder {
    match profile_service::get_profile_by_inbox_id(&pool, &inbox_id).await {
        Ok(profile) => HttpResponse::Ok().json(ProfileResponse::from(profile)),
        Err(_) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Profile not found"
        })),
    }
}

/// Claim a username
#[post("/claim")]
async fn claim_username(
    pool: web::Data<PgPool>,
    req: web::Json<ClaimUsernameRequest>,
) -> impl Responder {
    match profile_service::claim_username(&pool, &req.wallet_address, &req.username).await {
        Ok(profile) => HttpResponse::Ok().json(ProfileResponse::from(profile)),
        Err(e) => {
            log::warn!("Username claim failed: {}", e);
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": e.to_string()
            }))
        }
    }
}

/// Update profile
#[put("/update")]
async fn update_profile(
    pool: web::Data<PgPool>,
    req: web::Json<UpdateProfileRequest>,
) -> impl Responder {
    match profile_service::update_profile(&pool, req.into_inner()).await {
        Ok(profile) => HttpResponse::Ok().json(ProfileResponse::from(profile)),
        Err(e) => {
            log::warn!("Profile update failed: {}", e);
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": e.to_string()
            }))
        }
    }
}

/// Search profiles
#[get("/search")]
async fn search(
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
        .unwrap_or(10)
        .min(50); // Max 50 results
    
    match profile_service::search_profiles(&pool, q, limit).await {
        Ok(results) => HttpResponse::Ok().json(results),
        Err(e) => {
            log::error!("Search failed: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Search failed"
            }))
        }
    }
}

/// Check if username is available
#[get("/check/{username}")]
async fn check_username(
    pool: web::Data<PgPool>,
    username: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let wallet = query.get("wallet").map(|s| s.as_str()).unwrap_or("");
    
    // Validate format first
    if let Err(e) = profile_service::validate_username(&username) {
        return HttpResponse::Ok().json(serde_json::json!({
            "available": false,
            "reason": e.to_string()
        }));
    }
    
    match profile_service::is_username_available(&pool, &username, wallet).await {
        Ok(available) => {
            let mut response = serde_json::json!({
                "available": available
            });
            if !available {
                response["reason"] = serde_json::json!("Username is already taken");
            }
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            log::error!("Username check failed: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to check username"
            }))
        }
    }
}
