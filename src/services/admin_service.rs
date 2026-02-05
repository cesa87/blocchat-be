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
