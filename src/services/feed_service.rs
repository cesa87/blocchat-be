use sqlx::PgPool;
use uuid::Uuid;
use std::collections::HashMap;

use crate::models::feed::{
    FeedSubscription, FeedSnapshot, FeedEvent,
    CreateFeedSubscriptionRequest, UpdateFeedSubscriptionRequest,
    FeedSubscriptionResponse, FeedSnapshotResponse, FeedEventResponse,
    FeedStateResponse,
};

// ── Subscriptions ──

pub async fn create_subscription(
    pool: &PgPool,
    conversation_id: &str,
    req: CreateFeedSubscriptionRequest,
) -> Result<FeedSubscription, sqlx::Error> {
    let triggers_json = serde_json::to_value(req.triggers.unwrap_or_default())
        .unwrap_or_default();
    let poll_interval = req.poll_interval_secs.unwrap_or(60).max(30);

    sqlx::query_as::<_, FeedSubscription>(
        r#"INSERT INTO feed_subscriptions
             (conversation_id, feed_type, source_id, source_name, source_symbol,
              poll_interval_secs, triggers, created_by_wallet)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           ON CONFLICT (conversation_id, feed_type, source_id) DO UPDATE SET
             triggers = EXCLUDED.triggers,
             poll_interval_secs = EXCLUDED.poll_interval_secs,
             source_symbol = EXCLUDED.source_symbol,
             is_active = true,
             updated_at = NOW()
           RETURNING *"#,
    )
    .bind(conversation_id)
    .bind(&req.feed_type)
    .bind(&req.source_id)
    .bind(&req.source_name)
    .bind(&req.source_symbol)
    .bind(poll_interval)
    .bind(&triggers_json)
    .bind(&req.created_by_wallet)
    .fetch_one(pool)
    .await
}

pub async fn get_subscriptions_for_conversation(
    pool: &PgPool,
    conversation_id: &str,
) -> Result<Vec<FeedSubscription>, sqlx::Error> {
    sqlx::query_as::<_, FeedSubscription>(
        "SELECT * FROM feed_subscriptions WHERE conversation_id = $1 ORDER BY created_at ASC",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
}

pub async fn get_active_subscriptions(
    pool: &PgPool,
) -> Result<Vec<FeedSubscription>, sqlx::Error> {
    sqlx::query_as::<_, FeedSubscription>(
        "SELECT * FROM feed_subscriptions WHERE is_active = true",
    )
    .fetch_all(pool)
    .await
}

pub async fn update_subscription(
    pool: &PgPool,
    subscription_id: &Uuid,
    req: UpdateFeedSubscriptionRequest,
) -> Result<FeedSubscription, sqlx::Error> {
    let current = sqlx::query_as::<_, FeedSubscription>(
        "SELECT * FROM feed_subscriptions WHERE id = $1",
    )
    .bind(subscription_id)
    .fetch_one(pool)
    .await?;

    let triggers_json = match req.triggers {
        Some(t) => serde_json::to_value(t).unwrap_or(current.triggers.clone()),
        None => current.triggers.clone(),
    };
    let poll_interval = req.poll_interval_secs.unwrap_or(current.poll_interval_secs).max(30);
    let is_active = req.is_active.unwrap_or(current.is_active);

    sqlx::query_as::<_, FeedSubscription>(
        r#"UPDATE feed_subscriptions
           SET triggers = $1, poll_interval_secs = $2, is_active = $3, updated_at = NOW()
           WHERE id = $4
           RETURNING *"#,
    )
    .bind(&triggers_json)
    .bind(poll_interval)
    .bind(is_active)
    .bind(subscription_id)
    .fetch_one(pool)
    .await
}

pub async fn delete_subscription(
    pool: &PgPool,
    subscription_id: &Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM feed_subscriptions WHERE id = $1")
        .bind(subscription_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Snapshots ──

pub async fn insert_snapshot(
    pool: &PgPool,
    subscription_id: &Uuid,
    price: Option<f64>,
    price_prev: Option<f64>,
    pct_change_24h: Option<f64>,
    volume_24h: Option<f64>,
    market_cap: Option<f64>,
    candles: serde_json::Value,
    raw_data: serde_json::Value,
) -> Result<FeedSnapshot, sqlx::Error> {
    sqlx::query_as::<_, FeedSnapshot>(
        r#"INSERT INTO feed_snapshots
             (subscription_id, price, price_prev, pct_change_24h, volume_24h, market_cap, candles, raw_data)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING *"#,
    )
    .bind(subscription_id)
    .bind(price)
    .bind(price_prev)
    .bind(pct_change_24h)
    .bind(volume_24h)
    .bind(market_cap)
    .bind(&candles)
    .bind(&raw_data)
    .fetch_one(pool)
    .await
}

pub async fn get_latest_snapshot(
    pool: &PgPool,
    subscription_id: &Uuid,
) -> Result<Option<FeedSnapshot>, sqlx::Error> {
    sqlx::query_as::<_, FeedSnapshot>(
        "SELECT * FROM feed_snapshots WHERE subscription_id = $1 ORDER BY fetched_at DESC LIMIT 1",
    )
    .bind(subscription_id)
    .fetch_optional(pool)
    .await
}

/// Prune old snapshots — keep only the most recent N per subscription
pub async fn prune_snapshots(
    pool: &PgPool,
    subscription_id: &Uuid,
    keep: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"DELETE FROM feed_snapshots WHERE subscription_id = $1
           AND id NOT IN (
             SELECT id FROM feed_snapshots WHERE subscription_id = $1
             ORDER BY fetched_at DESC LIMIT $2
           )"#,
    )
    .bind(subscription_id)
    .bind(keep)
    .execute(pool)
    .await?;
    Ok(())
}

// ── Events ──

pub async fn insert_event(
    pool: &PgPool,
    subscription_id: &Uuid,
    conversation_id: &str,
    event_type: &str,
    title: &str,
    body: &str,
    metadata: serde_json::Value,
) -> Result<FeedEvent, sqlx::Error> {
    sqlx::query_as::<_, FeedEvent>(
        r#"INSERT INTO feed_events
             (subscription_id, conversation_id, event_type, title, body, metadata)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING *"#,
    )
    .bind(subscription_id)
    .bind(conversation_id)
    .bind(event_type)
    .bind(title)
    .bind(body)
    .bind(&metadata)
    .fetch_one(pool)
    .await
}

/// Check if an event of the same type was fired recently (cooldown enforcement)
pub async fn recent_event_exists(
    pool: &PgPool,
    subscription_id: &Uuid,
    event_type: &str,
    cooldown_secs: i64,
) -> Result<bool, sqlx::Error> {
    let row = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM feed_events
           WHERE subscription_id = $1
             AND event_type = $2
             AND created_at > NOW() - ($3 || ' seconds')::INTERVAL"#,
    )
    .bind(subscription_id)
    .bind(event_type)
    .bind(cooldown_secs)
    .fetch_one(pool)
    .await?;
    Ok(row > 0)
}

pub async fn get_unseen_events(
    pool: &PgPool,
    conversation_id: &str,
) -> Result<Vec<FeedEvent>, sqlx::Error> {
    sqlx::query_as::<_, FeedEvent>(
        r#"SELECT * FROM feed_events
           WHERE conversation_id = $1 AND is_seen = false
           ORDER BY created_at ASC"#,
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
}

pub async fn get_recent_events(
    pool: &PgPool,
    conversation_id: &str,
    limit: i64,
) -> Result<Vec<FeedEvent>, sqlx::Error> {
    sqlx::query_as::<_, FeedEvent>(
        r#"SELECT * FROM feed_events
           WHERE conversation_id = $1
           ORDER BY created_at DESC
           LIMIT $2"#,
    )
    .bind(conversation_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn mark_events_seen(
    pool: &PgPool,
    event_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE feed_events SET is_seen = true WHERE id = ANY($1)",
    )
    .bind(event_ids)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_event_seen(
    pool: &PgPool,
    event_id: &Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE feed_events SET is_seen = true WHERE id = $1")
        .bind(event_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── State (used by GET /state endpoint) ──

pub async fn get_feed_state(
    pool: &PgPool,
    conversation_id: &str,
) -> Result<FeedStateResponse, sqlx::Error> {
    let subscriptions = get_subscriptions_for_conversation(pool, conversation_id).await?;
    let unseen_events = get_unseen_events(pool, conversation_id).await?;

    let mut snapshots_by_subscription_id = HashMap::new();
    for sub in &subscriptions {
        if let Some(snapshot) = get_latest_snapshot(pool, &sub.id).await? {
            snapshots_by_subscription_id.insert(
                sub.id.to_string(),
                FeedSnapshotResponse::from(snapshot),
            );
        }
    }

    Ok(FeedStateResponse {
        subscriptions: subscriptions.into_iter().map(FeedSubscriptionResponse::from).collect(),
        snapshots_by_subscription_id,
        unseen_events: unseen_events.into_iter().map(FeedEventResponse::from).collect(),
    })
}
