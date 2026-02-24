-- Feeds: per-conversation data feed subscriptions, price snapshots, and trigger events

CREATE TABLE IF NOT EXISTS feed_subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id TEXT NOT NULL,
    feed_type TEXT NOT NULL,          -- 'crypto' (v1), 'stock', 'news', 'social' (future)
    source_id TEXT NOT NULL,          -- CoinGecko coin id e.g. 'bitcoin'
    source_name TEXT NOT NULL,        -- Human-readable e.g. 'Bitcoin'
    source_symbol TEXT NOT NULL DEFAULT '',  -- e.g. 'BTC'
    poll_interval_secs INTEGER NOT NULL DEFAULT 60,
    triggers JSONB NOT NULL DEFAULT '[]'::jsonb,  -- array of TriggerRule
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_by_wallet TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE (conversation_id, feed_type, source_id)
);

CREATE INDEX idx_feed_subscriptions_conversation ON feed_subscriptions(conversation_id);
CREATE INDEX idx_feed_subscriptions_active ON feed_subscriptions(is_active) WHERE is_active = true;

-- Latest price/data snapshots per subscription (retain last N rows, pruned by poller)
CREATE TABLE IF NOT EXISTS feed_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subscription_id UUID NOT NULL REFERENCES feed_subscriptions(id) ON DELETE CASCADE,
    price DOUBLE PRECISION,
    price_prev DOUBLE PRECISION,      -- price at previous snapshot (for crossing detection)
    pct_change_24h DOUBLE PRECISION,
    volume_24h DOUBLE PRECISION,
    market_cap DOUBLE PRECISION,
    candles JSONB NOT NULL DEFAULT '[]'::jsonb,  -- [{time, open, high, low, close}]
    raw_data JSONB NOT NULL DEFAULT '{}'::jsonb,
    fetched_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_feed_snapshots_subscription ON feed_snapshots(subscription_id, fetched_at DESC);

-- Events fired when a trigger rule is crossed
CREATE TABLE IF NOT EXISTS feed_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subscription_id UUID NOT NULL REFERENCES feed_subscriptions(id) ON DELETE CASCADE,
    conversation_id TEXT NOT NULL,
    event_type TEXT NOT NULL,         -- 'price_above', 'price_below', 'pct_change_24h_above'
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    is_seen BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_feed_events_conversation ON feed_events(conversation_id, created_at DESC);
CREATE INDEX idx_feed_events_unseen ON feed_events(is_seen) WHERE is_seen = false;
CREATE INDEX idx_feed_events_subscription ON feed_events(subscription_id, created_at DESC);

-- Auto-update updated_at on subscriptions
CREATE TRIGGER update_feed_subscriptions_updated_at BEFORE UPDATE ON feed_subscriptions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

COMMENT ON TABLE feed_subscriptions IS 'Per-conversation data feed subscriptions';
COMMENT ON TABLE feed_snapshots IS 'Polled price/data snapshots for feed subscriptions';
COMMENT ON TABLE feed_events IS 'Trigger-fired events for feed subscriptions';
