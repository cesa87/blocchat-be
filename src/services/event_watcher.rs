use ethers::prelude::*;
use ethers::abi::{Abi, Event, RawLog};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::models::alpha_bot::AlphaBotConfig;
use crate::services::alpha_bot_service;

const POLL_INTERVAL_SECS: u64 = 15;
const CONFIG_REFRESH_SECS: u64 = 60;
const MAX_BLOCK_RANGE: u64 = 2000;

/// Spawn the background event watcher. Call once from main.rs.
pub fn spawn(pool: PgPool, rpc_url: String) {
    tokio::spawn(async move {
        log::info!("🤖 Alpha Bot event watcher starting...");
        if let Err(e) = run_loop(pool, rpc_url).await {
            log::error!("Alpha Bot event watcher fatal error: {}", e);
        }
    });
}

async fn run_loop(pool: PgPool, rpc_url: String) -> anyhow::Result<()> {
    let provider = Provider::<Http>::try_from(&rpc_url)?
        .interval(Duration::from_secs(POLL_INTERVAL_SECS));
    let provider = Arc::new(provider);

    let mut last_config_refresh = std::time::Instant::now();
    let mut configs: Vec<AlphaBotConfig> = Vec::new();
    // Cache parsed ABIs per config id
    let mut abi_cache: HashMap<uuid::Uuid, Vec<Event>> = HashMap::new();

    loop {
        // Refresh configs periodically
        if last_config_refresh.elapsed() >= Duration::from_secs(CONFIG_REFRESH_SECS)
            || configs.is_empty()
        {
            match alpha_bot_service::get_all_active_configs(&pool).await {
                Ok(new_configs) => {
                    // Rebuild ABI cache for new/changed configs
                    abi_cache.clear();
                    for cfg in &new_configs {
                        let events = parse_events_from_config(cfg);
                        abi_cache.insert(cfg.id, events);
                    }
                    if new_configs.len() != configs.len() {
                        log::info!(
                            "🤖 Alpha Bot watching {} active config(s)",
                            new_configs.len()
                        );
                    }
                    configs = new_configs;
                    last_config_refresh = std::time::Instant::now();
                }
                Err(e) => {
                    log::error!("Failed to refresh alpha bot configs: {}", e);
                }
            }
        }

        if configs.is_empty() {
            tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
            continue;
        }

        // Get current block number
        let current_block = match provider.get_block_number().await {
            Ok(b) => b.as_u64(),
            Err(e) => {
                log::error!("Failed to get block number: {}", e);
                tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
                continue;
            }
        };

        // Process each config
        for cfg in &configs {
            let from_block = if cfg.last_block_checked > 0 {
                (cfg.last_block_checked as u64) + 1
            } else {
                // First run: start from a recent block (don't scan entire history)
                current_block.saturating_sub(10)
            };

            if from_block > current_block {
                continue;
            }

            // Cap range to avoid RPC limits
            let to_block = std::cmp::min(from_block + MAX_BLOCK_RANGE, current_block);

            let contract_address: Address = match cfg.contract_address.parse() {
                Ok(a) => a,
                Err(e) => {
                    log::warn!(
                        "Invalid contract address {} for config {}: {}",
                        cfg.contract_address, cfg.id, e
                    );
                    continue;
                }
            };

            // Build topic filters from configured event signatures
            let abi_events = abi_cache.get(&cfg.id).cloned().unwrap_or_default();
            let topic0_filters: Vec<H256> = abi_events
                .iter()
                .map(|ev| ev.signature())
                .collect();

            if topic0_filters.is_empty() {
                // No parseable events — fetch all logs for this contract
                let filter = Filter::new()
                    .address(contract_address)
                    .from_block(from_block)
                    .to_block(to_block);

                process_logs(&pool, &provider, cfg, &filter, &abi_events).await;
            } else {
                let filter = Filter::new()
                    .address(contract_address)
                    .topic0(topic0_filters)
                    .from_block(from_block)
                    .to_block(to_block);

                process_logs(&pool, &provider, cfg, &filter, &abi_events).await;
            }

            // Update checkpoint
            if let Err(e) =
                alpha_bot_service::update_last_block(&pool, &cfg.id, to_block as i64).await
            {
                log::error!("Failed to update last_block for config {}: {}", cfg.id, e);
            }
        }

        tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
    }
}

async fn process_logs(
    pool: &PgPool,
    provider: &Provider<Http>,
    cfg: &AlphaBotConfig,
    filter: &Filter,
    abi_events: &[Event],
) {
    let logs = match provider.get_logs(filter).await {
        Ok(l) => l,
        Err(e) => {
            log::error!(
                "Failed to fetch logs for config {} ({}): {}",
                cfg.id, cfg.contract_address, e
            );
            return;
        }
    };

    for log_entry in logs {
        let tx_hash = format!("{:#x}", log_entry.transaction_hash.unwrap_or_default());
        let block_number = log_entry.block_number.map(|b| b.as_u64() as i64).unwrap_or(0);
        let log_index = log_entry.log_index.map(|i| i.as_u32() as i32).unwrap_or(0);

        // Try to decode the log against known events
        let (event_name, decoded_data) = decode_log(&log_entry, abi_events);

        if let Err(e) = alpha_bot_service::insert_alert(
            pool,
            &cfg.id,
            &cfg.conversation_id,
            &event_name,
            &tx_hash,
            block_number,
            log_index,
            &decoded_data,
        )
        .await
        {
            // ON CONFLICT DO NOTHING returns RowNotFound, which is expected for duplicates
            if !matches!(e, sqlx::Error::RowNotFound) {
                log::error!("Failed to insert alert: {}", e);
            }
        }
    }
}

/// Attempt to decode a log entry against the configured ABI events.
/// Returns (event_name, decoded_params_as_json).
fn decode_log(log_entry: &Log, abi_events: &[Event]) -> (String, serde_json::Value) {
    let raw = RawLog {
        topics: log_entry.topics.clone(),
        data: log_entry.data.to_vec(),
    };

    for event in abi_events {
        if let Ok(decoded) = event.parse_log(raw.clone()) {
            let mut params = serde_json::Map::new();
            for param in decoded.params {
                let value_str = format!("{}", param.value);
                params.insert(param.name.clone(), serde_json::Value::String(value_str));
            }
            return (
                event.name.clone(),
                serde_json::Value::Object(params),
            );
        }
    }

    // Couldn't decode — return raw topic0 as event name
    let topic0 = log_entry
        .topics
        .first()
        .map(|t| format!("{:#x}", t))
        .unwrap_or_else(|| "Unknown".to_string());

    (
        topic0,
        serde_json::json!({
            "raw_data": format!("0x{}", hex::encode(&log_entry.data)),
            "topics": log_entry.topics.iter().map(|t| format!("{:#x}", t)).collect::<Vec<_>>(),
        }),
    )
}

/// Parse event ABI fragments from config.
/// Supports: full ABI JSON, or falls back to building from event signature strings.
fn parse_events_from_config(cfg: &AlphaBotConfig) -> Vec<Event> {
    let mut events = Vec::new();

    // Try parsing full ABI first
    if let Some(ref abi_str) = cfg.abi_json {
        if let Ok(abi) = serde_json::from_str::<Abi>(abi_str) {
            // Filter to only the events the user configured
            let configured: Vec<String> =
                serde_json::from_value(cfg.events.clone()).unwrap_or_default();

            for (name, event_variants) in &abi.events {
                for ev in event_variants {
                    // Match by name prefix (user might pass "Transfer" without full sig)
                    // Build the full signature string: "Name(type1,type2,...)"
                    let full_sig = format!(
                        "{}({})",
                        ev.name,
                        ev.inputs.iter().map(|p| p.kind.to_string()).collect::<Vec<_>>().join(",")
                    );
                    if configured.iter().any(|c| {
                        c == name || c == &full_sig || full_sig.starts_with(c)
                    }) {
                        events.push(ev.clone());
                    }
                }
            }

            if !events.is_empty() {
                return events;
            }

            // If no specific matches, return all events from ABI
            for event_variants in abi.events.values() {
                events.extend(event_variants.iter().cloned());
            }
            return events;
        }
    }

    // Fallback: try to parse event signature strings into ABI Event objects
    let configured: Vec<String> =
        serde_json::from_value(cfg.events.clone()).unwrap_or_default();

    for sig in &configured {
        // For full signatures like "Transfer(address,address,uint256)", use HumanReadableParser
        if let Ok(event) = ethers::abi::HumanReadableParser::parse_event(&format!("event {}", sig))
        {
            events.push(event);
        } else {
            log::warn!("Could not parse event signature: {}", sig);
        }
    }

    events
}
