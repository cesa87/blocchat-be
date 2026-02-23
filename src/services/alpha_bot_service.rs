use sqlx::PgPool;
use chrono::{DateTime, Utc};
use crate::models::alpha_bot::{
    AlphaBotConfig, AlphaBotAlert,
    CreateAlphaBotConfigRequest, UpdateAlphaBotConfigRequest,
};

// ── Config CRUD ──

pub async fn create_config(
    pool: &PgPool,
    conversation_id: &str,
    req: CreateAlphaBotConfigRequest,
) -> Result<AlphaBotConfig, sqlx::Error> {
    let events_json = serde_json::to_value(&req.events).unwrap_or_default();
    let chain_id = req.chain_id.unwrap_or(8453);

    sqlx::query_as::<_, AlphaBotConfig>(
        r#"INSERT INTO alpha_bot_configs
             (conversation_id, contract_address, chain_id, events, abi_json, created_by_wallet)
           VALUES ($1, $2, $3, $4, $5, $6)
           ON CONFLICT (conversation_id, contract_address) DO UPDATE SET
             events = EXCLUDED.events,
             abi_json = EXCLUDED.abi_json,
             chain_id = EXCLUDED.chain_id,
             is_active = true,
             updated_at = NOW()
           RETURNING *"#,
    )
    .bind(conversation_id)
    .bind(&req.contract_address)
    .bind(chain_id)
    .bind(&events_json)
    .bind(&req.abi_json)
    .bind(&req.created_by_wallet)
    .fetch_one(pool)
    .await
}

pub async fn get_configs_for_conversation(
    pool: &PgPool,
    conversation_id: &str,
) -> Result<Vec<AlphaBotConfig>, sqlx::Error> {
    sqlx::query_as::<_, AlphaBotConfig>(
        "SELECT * FROM alpha_bot_configs WHERE conversation_id = $1 ORDER BY created_at DESC",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
}

pub async fn get_config(
    pool: &PgPool,
    config_id: &uuid::Uuid,
) -> Result<AlphaBotConfig, sqlx::Error> {
    sqlx::query_as::<_, AlphaBotConfig>(
        "SELECT * FROM alpha_bot_configs WHERE id = $1",
    )
    .bind(config_id)
    .fetch_one(pool)
    .await
}

pub async fn update_config(
    pool: &PgPool,
    config_id: &uuid::Uuid,
    req: UpdateAlphaBotConfigRequest,
) -> Result<AlphaBotConfig, sqlx::Error> {
    // Fetch current to merge
    let current = get_config(pool, config_id).await?;

    let events_json = match &req.events {
        Some(evts) => serde_json::to_value(evts).unwrap_or(current.events.clone()),
        None => current.events.clone(),
    };
    let abi_json = req.abi_json.or(current.abi_json);
    let is_active = req.is_active.unwrap_or(current.is_active);
    let chain_id = req.chain_id.unwrap_or(current.chain_id);

    sqlx::query_as::<_, AlphaBotConfig>(
        r#"UPDATE alpha_bot_configs
           SET events = $1, abi_json = $2, is_active = $3, chain_id = $4, updated_at = NOW()
           WHERE id = $5
           RETURNING *"#,
    )
    .bind(&events_json)
    .bind(&abi_json)
    .bind(is_active)
    .bind(chain_id)
    .bind(config_id)
    .fetch_one(pool)
    .await
}

pub async fn delete_config(
    pool: &PgPool,
    config_id: &uuid::Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM alpha_bot_configs WHERE id = $1")
        .bind(config_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Active configs (used by event watcher) ──

pub async fn get_all_active_configs(
    pool: &PgPool,
) -> Result<Vec<AlphaBotConfig>, sqlx::Error> {
    sqlx::query_as::<_, AlphaBotConfig>(
        "SELECT * FROM alpha_bot_configs WHERE is_active = true",
    )
    .fetch_all(pool)
    .await
}

pub async fn update_last_block(
    pool: &PgPool,
    config_id: &uuid::Uuid,
    block_number: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE alpha_bot_configs SET last_block_checked = $1 WHERE id = $2",
    )
    .bind(block_number)
    .bind(config_id)
    .execute(pool)
    .await?;
    Ok(())
}

// ── Alerts ──

pub async fn insert_alert(
    pool: &PgPool,
    config_id: &uuid::Uuid,
    conversation_id: &str,
    event_name: &str,
    tx_hash: &str,
    block_number: i64,
    log_index: i32,
    decoded_data: &serde_json::Value,
) -> Result<AlphaBotAlert, sqlx::Error> {
    sqlx::query_as::<_, AlphaBotAlert>(
        r#"INSERT INTO alpha_bot_alerts
             (config_id, conversation_id, event_name, tx_hash, block_number, log_index, decoded_data)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           ON CONFLICT (tx_hash, log_index) DO NOTHING
           RETURNING *"#,
    )
    .bind(config_id)
    .bind(conversation_id)
    .bind(event_name)
    .bind(tx_hash)
    .bind(block_number)
    .bind(log_index)
    .bind(decoded_data)
    .fetch_one(pool)
    .await
}

pub async fn get_alerts_for_conversation(
    pool: &PgPool,
    conversation_id: &str,
    since: Option<DateTime<Utc>>,
    limit: i64,
) -> Result<Vec<AlphaBotAlert>, sqlx::Error> {
    match since {
        Some(since_ts) => {
            sqlx::query_as::<_, AlphaBotAlert>(
                r#"SELECT * FROM alpha_bot_alerts
                   WHERE conversation_id = $1 AND created_at > $2
                   ORDER BY created_at ASC
                   LIMIT $3"#,
            )
            .bind(conversation_id)
            .bind(since_ts)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as::<_, AlphaBotAlert>(
                r#"SELECT * FROM alpha_bot_alerts
                   WHERE conversation_id = $1
                   ORDER BY created_at DESC
                   LIMIT $2"#,
            )
            .bind(conversation_id)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
    }
}
