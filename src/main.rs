use actix_cors::Cors;
use actix_web::{web, App, HttpServer, middleware::Logger};
use dotenv::dotenv;
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, RwLock};

mod handlers;
mod models;
mod services;
mod db;
mod middleware;

use models::{NonceStore, SessionStore};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables
    dotenv().ok();
    
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_address = format!("{}:{}", host, port);
    
    log::info!("🚀 Starting BlocChat Backend on {}", bind_address);
    
    // Initialize database pool
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    
    let db_pool = db::create_pool(&database_url)
        .await
        .expect("Failed to create database pool");
    
    log::info!("✓ Database connection established");
    
    // Initialize session and nonce stores
    let session_store: SessionStore = Arc::new(RwLock::new(HashMap::new()));
    let nonce_store: NonceStore = Arc::new(RwLock::new(HashMap::new()));
    
    log::info!("✓ Session and nonce stores initialized");
    
    // Get CORS origins
    let cors_origins = env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:5173".to_string());
    
    HttpServer::new(move || {
        let cors_origins = cors_origins.clone();
        let cors = Cors::default()
            .allowed_origin_fn(move |origin, _req_head| {
                cors_origins
                    .split(',')
                    .any(|allowed| origin.as_bytes() == allowed.as_bytes())
            })
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                actix_web::http::header::AUTHORIZATION,
                actix_web::http::header::ACCEPT,
                actix_web::http::header::CONTENT_TYPE,
            ])
            .supports_credentials()
            .max_age(3600);
        
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(session_store.clone()))
            .app_data(web::Data::new(nonce_store.clone()))
            .wrap(Logger::default())
            .wrap(cors)
            .service(
                web::scope("/api")
                    .service(handlers::health::health_check)
                    .service(handlers::admin::configure())
                    .service(handlers::payments::configure())
                    .service(handlers::token_gates::configure())
                    .service(handlers::shops::configure())
            )
    })
    .bind(&bind_address)?
    .run()
    .await
}
