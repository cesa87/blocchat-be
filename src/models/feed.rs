use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// ── Trigger rule (stored as JSONB array in feed_subscriptions.triggers) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerRule {
    /// "price_above" | "price_below" | "pct_change_24h_above"
    pub rule_type: String,
    pub value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

// ── Database rows ──

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct FeedSubscription {
    pub id: Uuid,
    pub conversation_id: String,
    pub feed_type: String,
    pub source_id: String,
    pub source_name: String,
    pub source_symbol: String,
    pub poll_interval_secs: i32,
    pub triggers: serde_json::Value,
    pub is_active: bool,
    pub created_by_wallet: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct FeedSnapshot {
    pub id: Uuid,
    pub subscription_id: Uuid,
    pub price: Option<f64>,
    pub price_prev: Option<f64>,
    pub pct_change_24h: Option<f64>,
    pub volume_24h: Option<f64>,
    pub market_cap: Option<f64>,
    pub candles: serde_json::Value,
    pub raw_data: serde_json::Value,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct FeedEvent {
    pub id: Uuid,
    pub subscription_id: Uuid,
    pub conversation_id: String,
    pub event_type: String,
    pub title: String,
    pub body: String,
    pub metadata: serde_json::Value,
    pub is_seen: bool,
    pub created_at: DateTime<Utc>,
}

// ── Request types ──

#[derive(Debug, Deserialize)]
pub struct CreateFeedSubscriptionRequest {
    pub feed_type: String,
    pub source_id: String,
    pub source_name: String,
    pub source_symbol: String,
    pub poll_interval_secs: Option<i32>,
    pub triggers: Option<Vec<TriggerRule>>,
    pub created_by_wallet: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFeedSubscriptionRequest {
    pub triggers: Option<Vec<TriggerRule>>,
    pub poll_interval_secs: Option<i32>,
    pub is_active: Option<bool>,
}

// ── Response types ──

#[derive(Debug, Serialize)]
pub struct FeedSubscriptionResponse {
    pub id: String,
    pub conversation_id: String,
    pub feed_type: String,
    pub source_id: String,
    pub source_name: String,
    pub source_symbol: String,
    pub poll_interval_secs: i32,
    pub triggers: Vec<TriggerRule>,
    pub is_active: bool,
    pub created_by_wallet: String,
    pub created_at: String,
}

impl From<FeedSubscription> for FeedSubscriptionResponse {
    fn from(s: FeedSubscription) -> Self {
        let triggers: Vec<TriggerRule> = serde_json::from_value(s.triggers).unwrap_or_default();
        Self {
            id: s.id.to_string(),
            conversation_id: s.conversation_id,
            feed_type: s.feed_type,
            source_id: s.source_id,
            source_name: s.source_name,
            source_symbol: s.source_symbol,
            poll_interval_secs: s.poll_interval_secs,
            triggers,
            is_active: s.is_active,
            created_by_wallet: s.created_by_wallet,
            created_at: s.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FeedSnapshotResponse {
    pub id: String,
    pub subscription_id: String,
    pub price: Option<f64>,
    pub price_prev: Option<f64>,
    pub pct_change_24h: Option<f64>,
    pub volume_24h: Option<f64>,
    pub market_cap: Option<f64>,
    pub candles: serde_json::Value,
    pub fetched_at: String,
}

impl From<FeedSnapshot> for FeedSnapshotResponse {
    fn from(s: FeedSnapshot) -> Self {
        Self {
            id: s.id.to_string(),
            subscription_id: s.subscription_id.to_string(),
            price: s.price,
            price_prev: s.price_prev,
            pct_change_24h: s.pct_change_24h,
            volume_24h: s.volume_24h,
            market_cap: s.market_cap,
            candles: s.candles,
            fetched_at: s.fetched_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FeedEventResponse {
    pub id: String,
    pub subscription_id: String,
    pub conversation_id: String,
    pub event_type: String,
    pub title: String,
    pub body: String,
    pub metadata: serde_json::Value,
    pub is_seen: bool,
    pub created_at: String,
}

impl From<FeedEvent> for FeedEventResponse {
    fn from(e: FeedEvent) -> Self {
        Self {
            id: e.id.to_string(),
            subscription_id: e.subscription_id.to_string(),
            conversation_id: e.conversation_id,
            event_type: e.event_type,
            title: e.title,
            body: e.body,
            metadata: e.metadata,
            is_seen: e.is_seen,
            created_at: e.created_at.to_rfc3339(),
        }
    }
}

/// Returned by GET /state — full snapshot of a conversation's feed data
#[derive(Debug, Serialize)]
pub struct FeedStateResponse {
    pub subscriptions: Vec<FeedSubscriptionResponse>,
    pub snapshots_by_subscription_id: std::collections::HashMap<String, FeedSnapshotResponse>,
    pub unseen_events: Vec<FeedEventResponse>,
}
