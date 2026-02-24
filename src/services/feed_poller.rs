use sqlx::PgPool;
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

use crate::models::feed::{FeedSubscription, TriggerRule};
use crate::services::feed_service;

const SUBSCRIPTION_REFRESH_SECS: u64 = 60;
const POLL_LOOP_SLEEP_SECS: u64 = 10;
const SNAPSHOT_KEEP: i64 = 50;           // keep last 50 snapshots per subscription
const TRIGGER_COOLDOWN_SECS: i64 = 3600; // 1h cooldown per trigger type

/// Spawn the feed poller background task. Call once from main.rs.
pub fn spawn(pool: PgPool) {
    tokio::spawn(async move {
        log::info!("📡 Feed poller starting...");
        run_loop(pool).await;
    });
}

async fn run_loop(pool: PgPool) {
    let mut last_refresh = std::time::Instant::now()
        .checked_sub(Duration::from_secs(SUBSCRIPTION_REFRESH_SECS + 1))
        .unwrap_or(std::time::Instant::now());
    let mut subscriptions: Vec<FeedSubscription> = Vec::new();
    // Track when each subscription was last polled
    let mut last_poll_times: HashMap<Uuid, std::time::Instant> = HashMap::new();

    loop {
        // Refresh active subscriptions periodically
        if last_refresh.elapsed() >= Duration::from_secs(SUBSCRIPTION_REFRESH_SECS) {
            match feed_service::get_active_subscriptions(&pool).await {
                Ok(subs) => {
                    if subs.len() != subscriptions.len() {
                        log::info!("📡 Feed poller tracking {} active subscription(s)", subs.len());
                    }
                    subscriptions = subs;
                    last_refresh = std::time::Instant::now();
                }
                Err(e) => {
                    log::error!("Feed poller: failed to refresh subscriptions: {}", e);
                }
            }
        }

        for sub in &subscriptions {
            let interval = Duration::from_secs(sub.poll_interval_secs as u64);
            let should_poll = match last_poll_times.get(&sub.id) {
                Some(last) => last.elapsed() >= interval,
                None => true,
            };

            if should_poll {
                match poll_subscription(&pool, sub).await {
                    Ok(_) => {
                        last_poll_times.insert(sub.id, std::time::Instant::now());
                    }
                    Err(e) => {
                        log::error!(
                            "Feed poller: error polling subscription {} ({}:{}): {}",
                            sub.id, sub.feed_type, sub.source_id, e
                        );
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(POLL_LOOP_SLEEP_SECS)).await;
    }
}

async fn poll_subscription(pool: &PgPool, sub: &FeedSubscription) -> anyhow::Result<()> {
    match sub.feed_type.as_str() {
        "crypto"      => poll_crypto(pool, sub).await,
        "nft"         => poll_nft(pool, sub).await,
        "defi_tvl"    => poll_defi_tvl(pool, sub).await,
        "fear_greed"  => poll_fear_greed(pool, sub).await,
        other => {
            log::warn!("Feed poller: unsupported feed_type '{}', skipping", other);
            Ok(())
        }
    }
}

// ── CoinGecko crypto poller ──

#[derive(serde::Deserialize, Debug)]
struct CoinGeckoPrice {
    usd: Option<f64>,
    usd_24h_change: Option<f64>,
    usd_24h_vol: Option<f64>,
    usd_market_cap: Option<f64>,
}

async fn poll_crypto(pool: &PgPool, sub: &FeedSubscription) -> anyhow::Result<()> {
    // ── 1. Fetch simple price ──
    let price_url = format!(
        "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd\
         &include_24hr_change=true&include_24hr_vol=true&include_market_cap=true",
        sub.source_id
    );

    let cg_key_for_price = std::env::var("COINGECKO_API_KEY").ok();
    let mut price_req = reqwest::Client::new()
        .get(&price_url)
        .header("Accept", "application/json");
    if let Some(ref key) = cg_key_for_price {
        price_req = price_req.header("x-cg-demo-api-key", key);
    }
    let price_resp: serde_json::Value = price_req.send().await?.json().await?;

    let coin_data: CoinGeckoPrice = serde_json::from_value(
        price_resp.get(&sub.source_id).cloned().unwrap_or_default(),
    )
    .unwrap_or(CoinGeckoPrice {
        usd: None,
        usd_24h_change: None,
        usd_24h_vol: None,
        usd_market_cap: None,
    });

    let current_price = coin_data.usd;

    // ── 2. Fetch OHLC candles (daily, last 1 day = hourly candles) ──
    let cg_api_key = std::env::var("COINGECKO_API_KEY").ok();
    let candles = fetch_ohlc_candles(&sub.source_id, cg_api_key.as_deref())
        .await
        .unwrap_or_else(|e| {
            log::warn!("OHLC fetch failed for {}: {}", sub.source_id, e);
            serde_json::json!([])
        });

    // ── 3. Get previous snapshot price for crossing detection ──
    let prev_price = feed_service::get_latest_snapshot(pool, &sub.id)
        .await
        .ok()
        .flatten()
        .and_then(|s| s.price);

    // ── 4. Store snapshot ──
    let snapshot = feed_service::insert_snapshot(
        pool,
        &sub.id,
        current_price,
        prev_price,
        coin_data.usd_24h_change,
        coin_data.usd_24h_vol,
        coin_data.usd_market_cap,
        candles,
        price_resp.clone(),
    )
    .await?;

    // Prune old snapshots
    let _ = feed_service::prune_snapshots(pool, &sub.id, SNAPSHOT_KEEP).await;

    // ── 5. Evaluate trigger rules ──
    if let Some(cur_price) = current_price {
        let triggers: Vec<TriggerRule> =
            serde_json::from_value(sub.triggers.clone()).unwrap_or_default();

        for rule in &triggers {
            evaluate_trigger(
                pool,
                sub,
                rule,
                cur_price,
                prev_price,
                coin_data.usd_24h_change,
                &snapshot,
            )
            .await;
        }
    }

    log::debug!(
        "📡 Polled {} ({}) — price: {:?}",
        sub.source_name, sub.source_id, current_price
    );

    Ok(())
}

async fn fetch_ohlc_candles(coin_id: &str, api_key: Option<&str>) -> anyhow::Result<serde_json::Value> {
    // CoinGecko OHLC: returns [[timestamp_ms, open, high, low, close], ...]
    // days=1 gives hourly candles for the last 24h
    let url = format!(
        "https://api.coingecko.com/api/v3/coins/{}/ohlc?vs_currency=usd&days=1",
        coin_id
    );

    let mut req = reqwest::Client::new()
        .get(&url)
        .header("Accept", "application/json");
    if let Some(key) = api_key {
        req = req.header("x-cg-demo-api-key", key);
    }
    let resp = req.send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("CoinGecko OHLC returned {}", resp.status());
    }
    let raw: Vec<Vec<f64>> = resp.json().await?;

    // Convert to [{time, open, high, low, close}] for lightweight-charts
    let candles: Vec<serde_json::Value> = raw
        .into_iter()
        .map(|c| {
            let time_secs = (c.get(0).copied().unwrap_or(0.0) / 1000.0) as i64;
            serde_json::json!({
                "time": time_secs,
                "open": c.get(1).copied().unwrap_or(0.0),
                "high": c.get(2).copied().unwrap_or(0.0),
                "low": c.get(3).copied().unwrap_or(0.0),
                "close": c.get(4).copied().unwrap_or(0.0),
            })
        })
        .collect();

    Ok(serde_json::Value::Array(candles))
}

async fn evaluate_trigger(
    pool: &PgPool,
    sub: &FeedSubscription,
    rule: &TriggerRule,
    current_price: f64,
    prev_price: Option<f64>,
    pct_change_24h: Option<f64>,
    _snapshot: &crate::models::feed::FeedSnapshot,
) {
    let fired = match rule.rule_type.as_str() {
        "price_above" => {
            // Crossing: was below, now at or above threshold
            let crossed = prev_price
                .map(|p| p < rule.value && current_price >= rule.value)
                .unwrap_or(current_price >= rule.value);
            crossed
        }
        "price_below" => {
            // Crossing: was above, now at or below threshold
            let crossed = prev_price
                .map(|p| p > rule.value && current_price <= rule.value)
                .unwrap_or(current_price <= rule.value);
            crossed
        }
        "pct_change_24h_above" => {
            // Absolute 24h change exceeds threshold (either direction)
            pct_change_24h
                .map(|p| p.abs() >= rule.value)
                .unwrap_or(false)
        }
        _ => false,
    };

    if !fired {
        return;
    }

    // Check cooldown — don't re-fire the same rule type within TRIGGER_COOLDOWN_SECS
    let event_type = &rule.rule_type;
    match feed_service::recent_event_exists(pool, &sub.id, event_type, TRIGGER_COOLDOWN_SECS).await
    {
        Ok(true) => return, // still within cooldown
        Err(e) => {
            log::error!("Feed trigger cooldown check failed: {}", e);
            return;
        }
        Ok(false) => {}
    }

    // Build event title/body
    let label = rule
        .label
        .clone()
        .unwrap_or_else(|| rule.rule_type.replace('_', " "));

    let (title, body) = match rule.rule_type.as_str() {
        "price_above" => (
            format!("📈 {} crossed ${:.2}", sub.source_name, rule.value),
            format!(
                "{} ({}) is now ${:.2}, above your alert of ${:.2}",
                sub.source_name, sub.source_symbol, current_price, rule.value
            ),
        ),
        "price_below" => (
            format!("📉 {} dropped below ${:.2}", sub.source_name, rule.value),
            format!(
                "{} ({}) is now ${:.2}, below your alert of ${:.2}",
                sub.source_name, sub.source_symbol, current_price, rule.value
            ),
        ),
        "pct_change_24h_above" => {
            let pct = pct_change_24h.unwrap_or(0.0);
            let direction = if pct >= 0.0 { "up" } else { "down" };
            (
                format!(
                    "⚡ {} moved {:.1}% in 24h",
                    sub.source_name,
                    pct.abs()
                ),
                format!(
                    "{} ({}) is {} {:.1}% in the last 24h. Current price: ${:.2}",
                    sub.source_name, sub.source_symbol, direction, pct.abs(), current_price
                ),
            )
        }
        _ => (label.clone(), format!("{} trigger fired", label)),
    };

    let metadata = serde_json::json!({
        "source_id": sub.source_id,
        "source_name": sub.source_name,
        "source_symbol": sub.source_symbol,
        "current_price": current_price,
        "prev_price": prev_price,
        "pct_change_24h": pct_change_24h,
        "rule_type": rule.rule_type,
        "rule_value": rule.value,
    });

    match feed_service::insert_event(
        pool,
        &sub.id,
        &sub.conversation_id,
        event_type,
        &title,
        &body,
        metadata,
    )
    .await
    {
        Ok(_) => log::info!("📡 Feed trigger fired: {} for sub {}", title, sub.id),
        Err(e) => log::error!("Feed poller: failed to insert event: {}", e),
    }
}

// ═══════════════════════════════════════════════════════════
// NFT Floor Price — CoinGecko /nfts/{id}
// ═══════════════════════════════════════════════════════════

async fn poll_nft(pool: &PgPool, sub: &FeedSubscription) -> anyhow::Result<()> {
    let url = format!("https://api.coingecko.com/api/v3/nfts/{}", sub.source_id);

    let cg_key = std::env::var("COINGECKO_API_KEY").ok();
    let mut req = reqwest::Client::new()
        .get(&url)
        .header("Accept", "application/json");
    if let Some(ref key) = cg_key {
        req = req.header("x-cg-demo-api-key", key);
    }
    let resp = req.send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("CoinGecko NFT returned {}", resp.status());
    }
    let raw: serde_json::Value = resp.json().await?;

    let floor_price_usd = raw["floor_price"]["usd"].as_f64();
    let pct_change_24h   = raw["floor_price_in_usd_24h_percentage_change"].as_f64();
    let volume_24h       = raw["volume_24h"]["usd"].as_f64();
    let market_cap       = raw["market_cap"]["usd"].as_f64();

    let prev_price = feed_service::get_latest_snapshot(pool, &sub.id)
        .await.ok().flatten().and_then(|s| s.price);

    let snapshot = feed_service::insert_snapshot(
        pool, &sub.id,
        floor_price_usd, prev_price,
        pct_change_24h, volume_24h, market_cap,
        serde_json::json!([]),   // no OHLC on free tier
        raw.clone(),
    ).await?;

    let _ = feed_service::prune_snapshots(pool, &sub.id, SNAPSHOT_KEEP).await;

    if let Some(cur) = floor_price_usd {
        let triggers: Vec<TriggerRule> =
            serde_json::from_value(sub.triggers.clone()).unwrap_or_default();
        for rule in &triggers {
            evaluate_trigger(pool, sub, rule, cur, prev_price, pct_change_24h, &snapshot).await;
        }
    }

    log::debug!("📡 NFT {} — floor: {:?}", sub.source_name, floor_price_usd);
    Ok(())
}

// ═══════════════════════════════════════════════════════════
// DeFi Protocol TVL — DeFiLlama /protocol/{slug}
// ═══════════════════════════════════════════════════════════

async fn poll_defi_tvl(pool: &PgPool, sub: &FeedSubscription) -> anyhow::Result<()> {
    let url = format!("https://api.llama.fi/protocol/{}", sub.source_id);

    let resp = reqwest::Client::new()
        .get(&url)
        .header("Accept", "application/json")
        .send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("DeFiLlama returned {}", resp.status());
    }
    let raw: serde_json::Value = resp.json().await?;

    let tvl_history = raw["tvl"].as_array()
        .ok_or_else(|| anyhow::anyhow!("No tvl array in DeFiLlama response"))?;

    if tvl_history.is_empty() {
        anyhow::bail!("Empty TVL history for {}", sub.source_id);
    }

    let current_tvl = tvl_history.last()
        .and_then(|d| d["totalLiquidityUSD"].as_f64());
    let prev_tvl = if tvl_history.len() >= 2 {
        tvl_history[tvl_history.len() - 2]["totalLiquidityUSD"].as_f64()
    } else {
        None
    };
    let pct_change_24h = match (current_tvl, prev_tvl) {
        (Some(cur), Some(prev)) if prev > 0.0 => Some((cur - prev) / prev * 100.0),
        _ => None,
    };

    let candles = build_tvl_candles(tvl_history);

    let prev_price = feed_service::get_latest_snapshot(pool, &sub.id)
        .await.ok().flatten().and_then(|s| s.price);

    let snapshot = feed_service::insert_snapshot(
        pool, &sub.id,
        current_tvl, prev_price,
        pct_change_24h, None, None,
        candles,
        serde_json::json!({"protocol": sub.source_id}),
    ).await?;

    let _ = feed_service::prune_snapshots(pool, &sub.id, SNAPSHOT_KEEP).await;

    if let Some(cur) = current_tvl {
        let triggers: Vec<TriggerRule> =
            serde_json::from_value(sub.triggers.clone()).unwrap_or_default();
        for rule in &triggers {
            evaluate_trigger(pool, sub, rule, cur, prev_price, pct_change_24h, &snapshot).await;
        }
    }

    log::debug!("📡 TVL {} — {:?}", sub.source_name, current_tvl);
    Ok(())
}

fn build_tvl_candles(tvl_history: &[serde_json::Value]) -> serde_json::Value {
    let start = tvl_history.len().saturating_sub(31);
    let window = &tvl_history[start..];

    let candles: Vec<serde_json::Value> = window.windows(2)
        .map(|pair| {
            let time  = pair[1]["date"].as_i64().unwrap_or(0);
            let open  = pair[0]["totalLiquidityUSD"].as_f64().unwrap_or(0.0);
            let close = pair[1]["totalLiquidityUSD"].as_f64().unwrap_or(0.0);
            serde_json::json!({
                "time": time, "open": open,
                "high": open.max(close), "low": open.min(close), "close": close,
            })
        })
        .collect();

    serde_json::Value::Array(candles)
}

// ═══════════════════════════════════════════════════════════
// Fear & Greed Index — Alternative.me /fng/
// ═══════════════════════════════════════════════════════════

async fn poll_fear_greed(pool: &PgPool, sub: &FeedSubscription) -> anyhow::Result<()> {
    let resp = reqwest::Client::new()
        .get("https://api.alternative.me/fng/?limit=31")
        .header("Accept", "application/json")
        .send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("Fear & Greed API returned {}", resp.status());
    }
    let raw: serde_json::Value = resp.json().await?;

    let data = raw["data"].as_array()
        .ok_or_else(|| anyhow::anyhow!("No data in F&G response"))?;
    if data.is_empty() {
        anyhow::bail!("Empty F&G data");
    }

    // API returns newest-first
    let today        = &data[0];
    let current_val  = today["value"].as_str().and_then(|v| v.parse::<f64>().ok());
    let classification = today["value_classification"]
        .as_str().unwrap_or("Unknown").to_string();
    let prev_val     = data.get(1)
        .and_then(|d| d["value"].as_str())
        .and_then(|v| v.parse::<f64>().ok());

    let pct_change_24h = match (current_val, prev_val) {
        (Some(cur), Some(prev)) if prev > 0.0 => Some((cur - prev) / prev * 100.0),
        _ => None,
    };

    let candles = build_fng_candles(data);

    let prev_price = feed_service::get_latest_snapshot(pool, &sub.id)
        .await.ok().flatten().and_then(|s| s.price);

    let raw_meta = serde_json::json!({ "classification": classification });

    let snapshot = feed_service::insert_snapshot(
        pool, &sub.id,
        current_val, prev_price,
        pct_change_24h, None, None,
        candles,
        raw_meta,
    ).await?;

    let _ = feed_service::prune_snapshots(pool, &sub.id, SNAPSHOT_KEEP).await;

    if let Some(cur) = current_val {
        let triggers: Vec<TriggerRule> =
            serde_json::from_value(sub.triggers.clone()).unwrap_or_default();
        for rule in &triggers {
            evaluate_trigger(pool, sub, rule, cur, prev_price, pct_change_24h, &snapshot).await;
        }
    }

    log::debug!("📡 Fear & Greed — {:?} ({})", current_val, classification);
    Ok(())
}

fn build_fng_candles(data: &[serde_json::Value]) -> serde_json::Value {
    // data is newest-first; take up to 31, reverse for ascending time
    let mut asc: Vec<&serde_json::Value> = data.iter().take(31).collect();
    asc.reverse();

    let candles: Vec<serde_json::Value> = asc.windows(2)
        .map(|pair| {
            let time  = pair[1]["timestamp"].as_str()
                .and_then(|t| t.parse::<i64>().ok()).unwrap_or(0);
            let open  = pair[0]["value"].as_str()
                .and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
            let close = pair[1]["value"].as_str()
                .and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
            serde_json::json!({
                "time": time, "open": open,
                "high": open.max(close), "low": open.min(close), "close": close,
            })
        })
        .collect();

    serde_json::Value::Array(candles)
}
