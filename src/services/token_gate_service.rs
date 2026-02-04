use crate::db::DbPool;
use crate::models::{
    CreateTokenGateRequest, TokenGate, TokenGateResponse, TokenRequirementResponse,
    VerifyTokenGateRequest, VerifyTokenGateResponse, RequirementStatus,
};
use std::env;
use ethers::prelude::*;

// ERC-20 balanceOf ABI
abigen!(
    ERC20,
    r#"[
        function balanceOf(address owner) external view returns (uint256)
        function decimals() external view returns (uint8)
    ]"#,
);

pub async fn create_or_update_token_gates(
    pool: &DbPool,
    conversation_id: &str,
    req: CreateTokenGateRequest,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // Delete existing gates for this conversation
    sqlx::query("DELETE FROM token_gates WHERE conversation_id = $1")
        .bind(conversation_id)
        .execute(&mut *tx)
        .await?;

    // Insert new gates
    for requirement in req.requirements {
        sqlx::query(
            r#"
            INSERT INTO token_gates (conversation_id, token_address, token_symbol, min_amount, operator)
            VALUES ($1, $2, $3, $4, $5)
            "#
        )
        .bind(conversation_id)
        .bind(&requirement.token_address)
        .bind(&requirement.token_symbol)
        .bind(&requirement.min_amount)
        .bind(&req.operator)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn get_token_gates(
    pool: &DbPool,
    conversation_id: &str,
) -> Result<Option<TokenGateResponse>, sqlx::Error> {
    let gates: Vec<TokenGate> = sqlx::query_as(
        "SELECT * FROM token_gates WHERE conversation_id = $1 ORDER BY created_at ASC"
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await?;

    if gates.is_empty() {
        return Ok(None);
    }

    let operator = gates.first().unwrap().operator.clone();
    let requirements = gates.into_iter().map(TokenRequirementResponse::from).collect();

    Ok(Some(TokenGateResponse {
        requirements,
        operator,
    }))
}

pub async fn delete_token_gates(
    pool: &DbPool,
    conversation_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM token_gates WHERE conversation_id = $1")
        .bind(conversation_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn verify_token_gates(
    pool: &DbPool,
    req: VerifyTokenGateRequest,
) -> Result<VerifyTokenGateResponse, Box<dyn std::error::Error>> {
    // Get token gates for conversation
    let gates: Vec<TokenGate> = sqlx::query_as(
        "SELECT * FROM token_gates WHERE conversation_id = $1"
    )
    .bind(&req.conversation_id)
    .fetch_all(pool)
    .await?;

    if gates.is_empty() {
        // No gates, allow access
        return Ok(VerifyTokenGateResponse {
            allowed: true,
            requirements_met: vec![],
        });
    }

    let operator = gates.first().unwrap().operator.clone();
    let user_address: Address = req.wallet_address.parse()?;

    // Get Base RPC URL from env
    let rpc_url = env::var("BASE_RPC_URL")
        .unwrap_or_else(|_| "https://mainnet.base.org".to_string());
    let provider = Provider::<Http>::try_from(rpc_url)?;

    let mut requirements_met = Vec::new();
    let mut all_met = true;
    let mut any_met = false;

    for gate in gates {
        let balance_met = check_balance(
            &provider,
            gate.token_address.as_deref(),
            user_address,
            &gate.min_amount,
        )
        .await?;

        let status = RequirementStatus {
            token: gate.token_symbol.clone(),
            required: gate.min_amount.clone(),
            balance: balance_met.1.clone(),
            met: balance_met.0,
        };

        requirements_met.push(status);

        if balance_met.0 {
            any_met = true;
        } else {
            all_met = false;
        }
    }

    let allowed = match operator.as_str() {
        "AND" => all_met,
        "OR" => any_met,
        _ => false,
    };

    Ok(VerifyTokenGateResponse {
        allowed,
        requirements_met,
    })
}

async fn check_balance(
    provider: &Provider<Http>,
    token_address: Option<&str>,
    user_address: Address,
    min_amount: &str,
) -> Result<(bool, String), Box<dyn std::error::Error>> {
    let balance: U256 = if let Some(token_addr) = token_address {
        // ERC-20 token
        let token_address: Address = token_addr.parse()?;
        let contract = ERC20::new(token_address, provider.clone().into());
        contract.balance_of(user_address).call().await?
    } else {
        // Native ETH
        provider.get_balance(user_address, None).await?
    };

    // Convert balance to string for comparison
    let balance_str = balance.to_string();
    
    // Parse min_amount as U256
    let min_amount_u256: U256 = min_amount.parse()?;
    
    let met = balance >= min_amount_u256;

    Ok((met, balance_str))
}
