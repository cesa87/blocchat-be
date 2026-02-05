use actix_web::{cookie::Cookie, get, post, web, HttpResponse, Responder, Scope};
use sqlx::PgPool;
use std::env;

use crate::{
    models::{AuthRequest, AuthResponse, NonceRequest, NonceResponse, NonceStore, SessionStore},
    services::admin_service,
};

pub fn configure() -> Scope {
    web::scope("/admin")
        // Public auth endpoints
        .service(get_nonce)
        .service(authenticate)
        .service(check_auth)
        .service(logout)
        // Protected data endpoints (TODO: add middleware)
        .service(get_analytics)
        .service(get_transactions)
        .service(get_health)
}

/// Get a nonce for wallet signing
#[post("/nonce")]
async fn get_nonce(
    nonce_store: web::Data<NonceStore>,
    req: web::Json<NonceRequest>,
) -> impl Responder {
    let wallet_address = req.wallet_address.to_lowercase();
    
    // Generate nonce
    let nonce = admin_service::generate_nonce();
    
    // Create message to sign
    let message = format!(
        "Sign this message to authenticate with BlocChat Admin Dashboard.\n\nNonce: {}\n\nThis signature will not trigger any blockchain transaction or cost gas fees.",
        nonce
    );
    
    // Store nonce
    admin_service::store_nonce(&nonce_store, &wallet_address, nonce.clone());
    
    log::info!("Generated nonce for wallet: {}", wallet_address);
    
    HttpResponse::Ok().json(NonceResponse { nonce, message })
}

/// Authenticate with signed message
#[post("/auth")]
async fn authenticate(
    session_store: web::Data<SessionStore>,
    nonce_store: web::Data<NonceStore>,
    req: web::Json<AuthRequest>,
) -> impl Responder {
    let wallet_address = req.wallet_address.to_lowercase();
    
    log::info!("Authentication attempt from wallet: {}", wallet_address);
    
    // Check if wallet is in admin whitelist
    let admin_addresses = get_admin_addresses();
    if !admin_service::is_admin(&wallet_address, &admin_addresses) {
        log::warn!("Unauthorized admin attempt from: {}", wallet_address);
        return HttpResponse::Forbidden().json(AuthResponse {
            success: false,
            session_token: None,
            wallet_address: None,
        });
    }
    
    // Verify nonce
    if let Err(e) = admin_service::verify_nonce(&nonce_store, &wallet_address, &req.nonce) {
        log::warn!("Nonce verification failed for {}: {}", wallet_address, e);
        return HttpResponse::BadRequest().json(AuthResponse {
            success: false,
            session_token: None,
            wallet_address: None,
        });
    }
    
    // Create message that should have been signed
    let message = format!(
        "Sign this message to authenticate with BlocChat Admin Dashboard.\n\nNonce: {}\n\nThis signature will not trigger any blockchain transaction or cost gas fees.",
        req.nonce
    );
    
    // Verify signature
    match admin_service::verify_signature(&wallet_address, &message, &req.signature) {
        Ok(true) => {
            // Create session
            match admin_service::create_session(&session_store, &wallet_address) {
                Ok(session_token) => {
                    log::info!("Admin authenticated successfully: {}", wallet_address);
                    
                    // Set cookie
                    let cookie = Cookie::build("admin_session", session_token.clone())
                        .path("/")
                        .http_only(true)
                        .secure(true) // HTTPS only in production
                        .same_site(actix_web::cookie::SameSite::Strict)
                        .max_age(actix_web::cookie::time::Duration::days(1))
                        .finish();
                    
                    HttpResponse::Ok()
                        .cookie(cookie)
                        .json(AuthResponse {
                            success: true,
                            session_token: Some(session_token),
                            wallet_address: Some(wallet_address),
                        })
                }
                Err(e) => {
                    log::error!("Failed to create session: {}", e);
                    HttpResponse::InternalServerError().json(AuthResponse {
                        success: false,
                        session_token: None,
                        wallet_address: None,
                    })
                }
            }
        }
        Ok(false) => {
            log::warn!("Invalid signature from: {}", wallet_address);
            HttpResponse::Unauthorized().json(AuthResponse {
                success: false,
                session_token: None,
                wallet_address: None,
            })
        }
        Err(e) => {
            log::error!("Signature verification error: {}", e);
            HttpResponse::BadRequest().json(AuthResponse {
                success: false,
                session_token: None,
                wallet_address: None,
            })
        }
    }
}

/// Check if current session is valid
#[get("/check")]
async fn check_auth(session_store: web::Data<SessionStore>, req: actix_web::HttpRequest) -> impl Responder {
    // Extract token from cookie or header
    let token = extract_token(&req);
    
    if let Some(token) = token {
        match admin_service::verify_session(&session_store, &token) {
            Ok(wallet_address) => {
                return HttpResponse::Ok().json(serde_json::json!({
                    "authenticated": true,
                    "wallet_address": wallet_address
                }));
            }
            Err(_) => {}
        }
    }
    
    HttpResponse::Ok().json(serde_json::json!({
        "authenticated": false
    }))
}

/// Logout (invalidate session)
#[post("/logout")]
async fn logout(session_store: web::Data<SessionStore>, req: actix_web::HttpRequest) -> impl Responder {
    let token = extract_token(&req);
    
    if let Some(token) = token {
        let mut store = session_store.write().unwrap();
        store.remove(&token);
    }
    
    let cookie = Cookie::build("admin_session", "")
        .path("/")
        .max_age(actix_web::cookie::time::Duration::seconds(0))
        .finish();
    
    HttpResponse::Ok()
        .cookie(cookie)
        .json(serde_json::json!({
            "success": true
        }))
}

/// Helper to extract token from request
fn extract_token(req: &actix_web::HttpRequest) -> Option<String> {
    // Try Authorization header
    if let Some(auth_header) = req.headers().get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                return Some(auth_str[7..].to_string());
            }
        }
    }
    
    // Try cookie
    if let Some(cookie) = req.cookie("admin_session") {
        return Some(cookie.value().to_string());
    }
    
    None
}

/// Get admin addresses from environment variable
fn get_admin_addresses() -> Vec<String> {
    env::var("ADMIN_ADDRESSES")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

// ===== Protected Admin Data Endpoints =====

/// Get analytics data
#[get("/analytics")]
async fn get_analytics(pool: web::Data<PgPool>) -> impl Responder {
    match admin_service::get_analytics(&pool).await {
        Ok(analytics) => HttpResponse::Ok().json(analytics),
        Err(e) => {
            log::error!("Failed to get analytics: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch analytics"
            }))
        }
    }
}

/// Get recent transactions
#[get("/transactions")]
async fn get_transactions(
    pool: web::Data<PgPool>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let limit = query
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(50);
    
    match admin_service::get_recent_transactions(&pool, limit).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            log::error!("Failed to get transactions: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch transactions"
            }))
        }
    }
}

/// Get system health
#[get("/health")]
async fn get_health(pool: web::Data<PgPool>) -> impl Responder {
    match admin_service::get_system_health(&pool).await {
        Ok(health) => HttpResponse::Ok().json(health),
        Err(e) => {
            log::error!("Failed to get system health: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch system health"
            }))
        }
    }
}
