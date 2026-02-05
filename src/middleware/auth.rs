use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};

use crate::models::SessionStore;
use crate::services::admin_service;

// Middleware factory for admin authentication
pub struct AdminAuth {
    pub session_store: SessionStore,
}

impl<S, B> Transform<S, ServiceRequest> for AdminAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AdminAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AdminAuthMiddleware {
            service,
            session_store: self.session_store.clone(),
        }))
    }
}

pub struct AdminAuthMiddleware<S> {
    service: S,
    session_store: SessionStore,
}

impl<S, B> Service<ServiceRequest> for AdminAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let session_store = self.session_store.clone();
        
        // Extract session token from cookie or Authorization header
        let token = extract_token(&req);
        
        if let Some(token) = token {
            // Verify session
            match admin_service::verify_session(&session_store, &token) {
                Ok(wallet_address) => {
                    log::debug!("Admin authenticated: {}", wallet_address);
                    // Store wallet address in request extensions for handlers to access
                    req.extensions_mut().insert(wallet_address);
                    
                    let fut = self.service.call(req);
                    Box::pin(async move {
                        let res = fut.await?;
                        Ok(res)
                    })
                }
                Err(e) => {
                    log::warn!("Authentication failed: {}", e);
                    let response = HttpResponse::Unauthorized()
                        .json(serde_json::json!({
                            "error": "Unauthorized",
                            "message": "Invalid or expired session"
                        }));
                    
                    Box::pin(async move {
                        Err(actix_web::error::ErrorUnauthorized("Invalid or expired session"))
                    })
                }
            }
        } else {
            log::warn!("No authentication token provided");
            let response = HttpResponse::Unauthorized()
                .json(serde_json::json!({
                    "error": "Unauthorized",
                    "message": "Authentication required"
                }));
            
            Box::pin(async move {
                Err(actix_web::error::ErrorUnauthorized("Authentication required"))
            })
        }
    }
}

/// Extract session token from Cookie or Authorization header
fn extract_token(req: &ServiceRequest) -> Option<String> {
    // Try Authorization header first (Bearer token)
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
