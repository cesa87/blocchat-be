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
        "crypto" => poll_crypto(pool, sub).await,
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

    let price_resp: serde_json::Value = reqwest::Client::new()
        .get(&price_url)
        .header("Accept", "application/json")
        .send()
        .await?
        .json()
        .await?;

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
    let candles = fetch_ohlc_candles(&sub.source_id).await.unwrap_or_default();

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

async fn fetch_ohlc_candles(coin_id: &str) -> anyhow::Result<serde_json::Value> {
    // CoinGecko OHLC: returns [[timestamp_ms, open, high, low, close], ...]
    // days=1 gives hourly candles for the last 24h
    let url = format!(
        "https://api.coingecko.com/api/v3/coins/{}/ohlc?vs_currency=usd&days=1",
        coin_id
    );

    let raw: Vec<Vec<f64>> = reqwest::Client::new()
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?
        .json()
        .await?;

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
