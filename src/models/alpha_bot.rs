use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// ── Database rows ──

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AlphaBotConfig {
    pub id: Uuid,
    pub conversation_id: String,
    pub contract_address: String,
    pub chain_id: i32,
    pub events: serde_json::Value,      // JSONB array of event signatures
    pub abi_json: Option<String>,
    pub is_active: bool,
    pub created_by_wallet: String,
    pub last_block_checked: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AlphaBotAlert {
    pub id: Uuid,
    pub config_id: Uuid,
    pub conversation_id: String,
    pub event_name: String,
    pub tx_hash: String,
    pub block_number: i64,
    pub log_index: i32,
    pub decoded_data: serde_json::Value, // JSONB with decoded event params
    pub created_at: DateTime<Utc>,
}

// ── Request types ──

#[derive(Debug, Deserialize)]
pub struct CreateAlphaBotConfigRequest {
    pub contract_address: String,
    pub chain_id: Option<i32>,
    pub events: Vec<String>,             // e.g. ["Transfer(address,address,uint256)"]
    pub abi_json: Option<String>,
    pub created_by_wallet: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAlphaBotConfigRequest {
    pub events: Option<Vec<String>>,
    pub abi_json: Option<String>,
    pub is_active: Option<bool>,
    pub chain_id: Option<i32>,
}

// ── Response types ──

#[derive(Debug, Serialize)]
pub struct AlphaBotConfigResponse {
    pub id: String,
    pub conversation_id: String,
    pub contract_address: String,
    pub chain_id: i32,
    pub events: Vec<String>,
    pub has_abi: bool,
    pub is_active: bool,
    pub created_by_wallet: String,
    pub created_at: String,
}

impl From<AlphaBotConfig> for AlphaBotConfigResponse {
    fn from(c: AlphaBotConfig) -> Self {
        let events: Vec<String> = serde_json::from_value(c.events).unwrap_or_default();
        Self {
            id: c.id.to_string(),
            conversation_id: c.conversation_id,
            contract_address: c.contract_address,
            chain_id: c.chain_id,
            events,
            has_abi: c.abi_json.is_some(),
            is_active: c.is_active,
            created_by_wallet: c.created_by_wallet,
            created_at: c.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AlphaBotAlertResponse {
    pub id: String,
    pub event_name: String,
    pub tx_hash: String,
    pub block_number: i64,
    pub log_index: i32,
    pub decoded_data: serde_json::Value,
    pub created_at: String,
}

impl From<AlphaBotAlert> for AlphaBotAlertResponse {
    fn from(a: AlphaBotAlert) -> Self {
        Self {
            id: a.id.to_string(),
            event_name: a.event_name,
            tx_hash: a.tx_hash,
            block_number: a.block_number,
            log_index: a.log_index,
            decoded_data: a.decoded_data,
            created_at: a.created_at.to_rfc3339(),
        }
    }
}
