use crate::models::{AdminSession, NonceData, NonceStore, SessionStore};
use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use ethers::core::types::Signature;
use ethers::prelude::*;
use rand::Rng;
use sha3::{Digest, Keccak256};
use std::str::FromStr;

const SESSION_DURATION_HOURS: i64 = 24;
const NONCE_DURATION_MINUTES: i64 = 5;

/// Generate a random nonce for wallet signing
pub fn generate_nonce() -> String {
    let mut rng = rand::thread_rng();
    let nonce: u64 = rng.gen();
    format!("{:016x}", nonce)
}

/// Store a nonce for a wallet address
pub fn store_nonce(nonce_store: &NonceStore, wallet_address: &str, nonce: String) {
    let mut store = nonce_store.write().unwrap();
    store.insert(
        wallet_address.to_lowercase(),
        NonceData {
            nonce,
            created_at: Utc::now(),
        },
    );
}

/// Verify the nonce is valid and not expired
pub fn verify_nonce(nonce_store: &NonceStore, wallet_address: &str, nonce: &str) -> Result<()> {
    let mut store = nonce_store.write().unwrap();
    let wallet_key = wallet_address.to_lowercase();
    
    let nonce_data = store
        .get(&wallet_key)
        .ok_or_else(|| anyhow!("Nonce not found for wallet"))?;
    
    // Check if nonce matches
    if nonce_data.nonce != nonce {
        return Err(anyhow!("Invalid nonce"));
    }
    
    // Check if nonce has expired (5 minutes)
    let now = Utc::now();
    if now.signed_duration_since(nonce_data.created_at) > Duration::minutes(NONCE_DURATION_MINUTES) {
        store.remove(&wallet_key);
        return Err(anyhow!("Nonce expired"));
    }
    
    // Remove nonce after verification (one-time use)
    store.remove(&wallet_key);
    Ok(())
}

/// Verify an Ethereum signature (EIP-191)
pub fn verify_signature(wallet_address: &str, message: &str, signature: &str) -> Result<bool> {
    // Parse signature
    let sig = Signature::from_str(signature)
        .map_err(|e| anyhow!("Invalid signature format: {}", e))?;
    
    // Create EIP-191 message hash
    let message_hash = hash_message(message);
    
    // Recover the address from the signature
    let recovered_address = sig.recover(message_hash)
        .map_err(|e| anyhow!("Failed to recover address: {}", e))?;
    
    // Parse expected address
    let expected_address = wallet_address.parse::<Address>()
        .map_err(|e| anyhow!("Invalid wallet address: {}", e))?;
    
    // Compare addresses (case-insensitive)
    Ok(recovered_address.to_string().to_lowercase() == expected_address.to_string().to_lowercase())
}

/// Hash a message using Keccak256 with EIP-191 prefix
fn hash_message(message: &str) -> [u8; 32] {
    let message_bytes = message.as_bytes();
    let eth_message = format!("\x19Ethereum Signed Message:\n{}{}", message_bytes.len(), message);
    
    let mut hasher = Keccak256::new();
    hasher.update(eth_message.as_bytes());
    let result = hasher.finalize();
    
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Create a new admin session
pub fn create_session(
    session_store: &SessionStore,
    wallet_address: &str,
) -> Result<String> {
    let session_token = generate_session_token();
    let now = Utc::now();
    
    let session = AdminSession {
        wallet_address: wallet_address.to_lowercase(),
        created_at: now,
        expires_at: now + Duration::hours(SESSION_DURATION_HOURS),
    };
    
    let mut store = session_store.write().unwrap();
    store.insert(session_token.clone(), session);
    
    Ok(session_token)
}

/// Generate a secure session token
fn generate_session_token() -> String {
    let mut rng = rand::thread_rng();
    let token: [u8; 32] = rng.gen();
    hex::encode(token)
}

/// Verify a session token and return the wallet address
pub fn verify_session(session_store: &SessionStore, token: &str) -> Result<String> {
    let mut store = session_store.write().unwrap();
    
    let session = store
        .get(token)
        .ok_or_else(|| anyhow!("Invalid session token"))?
        .clone();
    
    // Check if session has expired
    let now = Utc::now();
    if now > session.expires_at {
        store.remove(token);
        return Err(anyhow!("Session expired"));
    }
    
    Ok(session.wallet_address)
}

/// Check if a wallet address is in the admin whitelist
pub fn is_admin(wallet_address: &str, admin_addresses: &[String]) -> bool {
    let addr_lower = wallet_address.to_lowercase();
    admin_addresses.iter().any(|admin| admin.to_lowercase() == addr_lower)
}

/// Clean up expired sessions and nonces
pub fn cleanup_expired(session_store: &SessionStore, nonce_store: &NonceStore) {
    let now = Utc::now();
    
    // Clean sessions
    {
        let mut sessions = session_store.write().unwrap();
        sessions.retain(|_, session| now <= session.expires_at);
    }
    
    // Clean nonces
    {
        let mut nonces = nonce_store.write().unwrap();
        nonces.retain(|_, nonce_data| {
            now.signed_duration_since(nonce_data.created_at) <= Duration::minutes(NONCE_DURATION_MINUTES)
        });
    }
}

/// Get analytics data from database
pub async fn get_analytics(pool: &sqlx::PgPool) -> Result<crate::models::AnalyticsResponse> {
    use crate::models::*;
    
    // Get transaction metrics
    let tx_metrics = sqlx::query!(
        r#"
        SELECT 
            COUNT(*) as total,
            SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) as pending,
            SUM(CASE WHEN status = 'confirmed' THEN 1 ELSE 0 END) as confirmed,
            SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed
        FROM transactions
        "#
    )
    .fetch_one(pool)
    .await?;
    
    // Get unique user count (distinct addresses)
    let user_count = sqlx::query!(
        r#"
        SELECT COUNT(DISTINCT from_address) as total FROM transactions
        "#
    )
    .fetch_one(pool)
    .await?;
    
    // Get platform metrics
    let platform_metrics = sqlx::query!(
        r#"
        SELECT 
            (SELECT COUNT(*) FROM token_gates) as token_gates,
            (SELECT COUNT(*) FROM shops) as shops,
            (SELECT COUNT(*) FROM shop_items) as shop_items
        "#
    )
    .fetch_one(pool)
    .await?;
    
    Ok(AnalyticsResponse {
        users: UserMetrics {
            total_users: user_count.total.unwrap_or(0),
            active_24h: 0, // TODO: implement with user activity tracking
            active_7d: 0,
        },
        transactions: TransactionMetrics {
            total_transactions: tx_metrics.total.unwrap_or(0),
            total_volume_usd: 0.0, // TODO: implement with price oracle
            pending: tx_metrics.pending.unwrap_or(0),
            confirmed: tx_metrics.confirmed.unwrap_or(0),
            failed: tx_metrics.failed.unwrap_or(0),
        },
        platform: PlatformMetrics {
            active_escrows: 0, // TODO: query from smart contract
            token_gates: platform_metrics.token_gates.unwrap_or(0),
            shops: platform_metrics.shops.unwrap_or(0),
            shop_items: platform_metrics.shop_items.unwrap_or(0),
        },
    })
}

/// Get recent transactions for admin view
pub async fn get_recent_transactions(
    pool: &sqlx::PgPool,
    limit: i64,
) -> Result<crate::models::RecentTransactionsResponse> {
    use crate::models::*;
    
    let transactions = sqlx::query_as::<_, Transaction>(
        r#"
        SELECT 
            id, tx_hash, from_address, to_address, amount, 
            token_address, chain_id, conversation_id, message_id,
            status, block_number, created_at, confirmed_at
        FROM transactions
        ORDER BY created_at DESC
        LIMIT $1
        "#
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    
    let total = sqlx::query!("SELECT COUNT(*) as count FROM transactions")
        .fetch_one(pool)
        .await?
        .count
        .unwrap_or(0);
    
    let transaction_views: Vec<AdminTransactionView> = transactions
        .into_iter()
        .map(|tx| AdminTransactionView {
            id: tx.id.to_string(),
            tx_hash: tx.tx_hash,
            from_address: tx.from_address,
            to_address: tx.to_address,
            amount: tx.amount,
            token_address: tx.token_address,
            status: format!("{:?}", tx.status).to_lowercase(),
            created_at: tx.created_at.to_rfc3339(),
            conversation_id: tx.conversation_id,
        })
        .collect();
    
    Ok(RecentTransactionsResponse {
        transactions: transaction_views,
        total,
    })
}

/// Get system health metrics
pub async fn get_system_health(pool: &sqlx::PgPool) -> Result<crate::models::SystemHealthResponse> {
    use crate::models::*;
    use std::time::SystemTime;
    
    // Get database size
    let db_size = sqlx::query!(
        "SELECT pg_database_size(current_database()) as size"
    )
    .fetch_one(pool)
    .await?;
    
    // Get table counts
    let table_counts = sqlx::query!(
        r#"
        SELECT 
            (SELECT COUNT(*) FROM transactions) as transactions,
            (SELECT COUNT(*) FROM token_gates) as token_gates,
            (SELECT COUNT(*) FROM shops) as shops,
            (SELECT COUNT(*) FROM shop_items) as shop_items
        "#
    )
    .fetch_one(pool)
    .await?;
    
    // Get connection count
    let connections = sqlx::query!(
        "SELECT COUNT(*) as count FROM pg_stat_activity WHERE datname = current_database()"
    )
    .fetch_one(pool)
    .await?;
    
    // Calculate uptime (placeholder - in production, track service start time)
    let uptime = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    Ok(SystemHealthResponse {
        backend: ServiceStatus {
            status: "healthy".to_string(),
            uptime_seconds: uptime % 86400, // Reset daily for demo
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        database: DatabaseStatus {
            status: "healthy".to_string(),
            connections: connections.count.unwrap_or(0) as i32,
            size_mb: db_size.size.unwrap_or(0) as f64 / (1024.0 * 1024.0),
            table_counts: TableCounts {
                transactions: table_counts.transactions.unwrap_or(0),
                token_gates: table_counts.token_gates.unwrap_or(0),
                shops: table_counts.shops.unwrap_or(0),
                shop_items: table_counts.shop_items.unwrap_or(0),
            },
        },
        resources: ResourceStatus {
            disk_usage_percent: 0.0, // TODO: implement with system metrics
            memory_mb: 0,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_nonce() {
        let nonce1 = generate_nonce();
        let nonce2 = generate_nonce();
        assert_ne!(nonce1, nonce2);
        assert_eq!(nonce1.len(), 16);
    }
    
    #[test]
    fn test_hash_message() {
        let message = "Hello, BlocChat!";
        let hash = hash_message(message);
        assert_eq!(hash.len(), 32);
    }
}
