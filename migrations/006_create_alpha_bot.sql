-- Alpha Bot: on-chain event watcher configs and alerts

CREATE TABLE IF NOT EXISTS alpha_bot_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id TEXT NOT NULL,
    contract_address TEXT NOT NULL,
    chain_id INTEGER NOT NULL DEFAULT 8453,
    events JSONB NOT NULL DEFAULT '[]'::jsonb,
    abi_json TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_by_wallet TEXT NOT NULL,
    last_block_checked BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE (conversation_id, contract_address)
);

CREATE INDEX idx_alpha_bot_configs_conversation ON alpha_bot_configs(conversation_id);
CREATE INDEX idx_alpha_bot_configs_active ON alpha_bot_configs(is_active) WHERE is_active = true;

CREATE TABLE IF NOT EXISTS alpha_bot_alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    config_id UUID NOT NULL REFERENCES alpha_bot_configs(id) ON DELETE CASCADE,
    conversation_id TEXT NOT NULL,
    event_name TEXT NOT NULL,
    tx_hash TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    log_index INTEGER NOT NULL,
    decoded_data JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE (tx_hash, log_index)
);

CREATE INDEX idx_alpha_bot_alerts_conversation ON alpha_bot_alerts(conversation_id);
CREATE INDEX idx_alpha_bot_alerts_config ON alpha_bot_alerts(config_id);
CREATE INDEX idx_alpha_bot_alerts_created ON alpha_bot_alerts(created_at DESC);

-- Auto-update updated_at on configs
CREATE TRIGGER update_alpha_bot_configs_updated_at BEFORE UPDATE ON alpha_bot_configs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

COMMENT ON TABLE alpha_bot_configs IS 'Alpha Bot watcher configurations per group conversation';
COMMENT ON TABLE alpha_bot_alerts IS 'Detected on-chain events from Alpha Bot watchers';
