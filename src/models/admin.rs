use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// Request to get a nonce for signing
#[derive(Debug, Deserialize)]
pub struct NonceRequest {
    pub wallet_address: String,
}

// Response containing nonce
#[derive(Debug, Serialize)]
pub struct NonceResponse {
    pub nonce: String,
    pub message: String,
}

// Request to authenticate with signed message
#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    pub wallet_address: String,
    pub signature: String,
    pub nonce: String,
}

// Response after successful authentication
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub success: bool,
    pub session_token: Option<String>,
    pub wallet_address: Option<String>,
}

// Admin session data
#[derive(Debug, Clone)]
pub struct AdminSession {
    pub wallet_address: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

// In-memory session store (for production, use Redis or database)
pub type SessionStore = Arc<RwLock<HashMap<String, AdminSession>>>;

// In-memory nonce store (expires after 5 minutes)
#[derive(Debug, Clone)]
pub struct NonceData {
    pub nonce: String,
    pub created_at: DateTime<Utc>,
}

pub type NonceStore = Arc<RwLock<HashMap<String, NonceData>>>;
