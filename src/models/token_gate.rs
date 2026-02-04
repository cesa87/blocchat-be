use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TokenGate {
    pub id: Uuid,
    pub conversation_id: String,
    pub token_address: Option<String>,
    pub token_symbol: String,
    pub min_amount: String,
    pub operator: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct TokenRequirement {
    pub token_address: Option<String>,
    pub token_symbol: String,
    pub min_amount: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTokenGateRequest {
    pub requirements: Vec<TokenRequirement>,
    pub operator: String,  // "AND" or "OR"
}

#[derive(Debug, Serialize)]
pub struct TokenGateResponse {
    pub requirements: Vec<TokenRequirementResponse>,
    pub operator: String,
}

#[derive(Debug, Serialize)]
pub struct TokenRequirementResponse {
    pub token_address: Option<String>,
    pub token_symbol: String,
    pub min_amount: String,
}

impl From<TokenGate> for TokenRequirementResponse {
    fn from(gate: TokenGate) -> Self {
        Self {
            token_address: gate.token_address,
            token_symbol: gate.token_symbol,
            min_amount: gate.min_amount,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct VerifyTokenGateRequest {
    pub conversation_id: String,
    pub wallet_address: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyTokenGateResponse {
    pub allowed: bool,
    pub requirements_met: Vec<RequirementStatus>,
}

#[derive(Debug, Serialize)]
pub struct RequirementStatus {
    pub token: String,
    pub required: String,
    pub balance: String,
    pub met: bool,
}
