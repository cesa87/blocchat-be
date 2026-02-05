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

// Analytics response types
#[derive(Debug, Serialize)]
pub struct AnalyticsResponse {
    pub users: UserMetrics,
    pub transactions: TransactionMetrics,
    pub platform: PlatformMetrics,
}

#[derive(Debug, Serialize)]
pub struct UserMetrics {
    pub total_users: i64,
    pub active_24h: i64,
    pub active_7d: i64,
}

#[derive(Debug, Serialize)]
pub struct TransactionMetrics {
    pub total_transactions: i64,
    pub total_volume_usd: f64,
    pub pending: i64,
    pub confirmed: i64,
    pub failed: i64,
}

#[derive(Debug, Serialize)]
pub struct PlatformMetrics {
    pub active_escrows: i64,
    pub token_gates: i64,
    pub shops: i64,
    pub shop_items: i64,
}

// System health response
#[derive(Debug, Serialize)]
pub struct SystemHealthResponse {
    pub backend: ServiceStatus,
    pub database: DatabaseStatus,
    pub resources: ResourceStatus,
}

#[derive(Debug, Serialize)]
pub struct ServiceStatus {
    pub status: String,
    pub uptime_seconds: u64,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct DatabaseStatus {
    pub status: String,
    pub connections: i32,
    pub size_mb: f64,
    pub table_counts: TableCounts,
}

#[derive(Debug, Serialize)]
pub struct TableCounts {
    pub transactions: i64,
    pub token_gates: i64,
    pub shops: i64,
    pub shop_items: i64,
}

#[derive(Debug, Serialize)]
pub struct ResourceStatus {
    pub disk_usage_percent: f32,
    pub memory_mb: u64,
}

// Recent transactions response
#[derive(Debug, Serialize)]
pub struct RecentTransactionsResponse {
    pub transactions: Vec<AdminTransactionView>,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct AdminTransactionView {
    pub id: String,
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub token_address: Option<String>,
    pub status: String,
    pub created_at: String,
    pub conversation_id: String,
}
