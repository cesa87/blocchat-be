use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Transaction {
    pub id: Uuid,
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: String, // Store as string to avoid precision issues
    pub token_address: Option<String>, // None for native ETH/Base
    pub chain_id: i32,
    pub conversation_id: String, // XMTP conversation ID
    pub message_id: Option<String>, // XMTP message ID if sent via XMTP
    pub status: TransactionStatus,
    pub block_number: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "transaction_status", rename_all = "lowercase")]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
}

#[derive(Debug, Deserialize)]
pub struct CreateTransactionRequest {
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub token_address: Option<String>,
    pub chain_id: i32,
    pub conversation_id: String,
    pub message_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TransactionResponse {
    pub id: Uuid,
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub token_address: Option<String>,
    pub chain_id: i32,
    pub status: TransactionStatus,
    pub block_number: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
}

impl From<Transaction> for TransactionResponse {
    fn from(tx: Transaction) -> Self {
        Self {
            id: tx.id,
            tx_hash: tx.tx_hash,
            from_address: tx.from_address,
            to_address: tx.to_address,
            amount: tx.amount,
            token_address: tx.token_address,
            chain_id: tx.chain_id,
            status: tx.status,
            block_number: tx.block_number,
            created_at: tx.created_at,
            confirmed_at: tx.confirmed_at,
        }
    }
}
