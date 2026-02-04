-- Create token_gates table
CREATE TABLE IF NOT EXISTS token_gates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id TEXT NOT NULL,
    token_address VARCHAR(42),  -- NULL for native ETH, otherwise ERC-20 address
    token_symbol VARCHAR(20) NOT NULL,
    min_amount TEXT NOT NULL,   -- Store as string to handle large numbers with decimals
    operator VARCHAR(3) NOT NULL DEFAULT 'AND',  -- AND or OR for multiple requirements
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create indexes for efficient queries
CREATE INDEX idx_token_gates_conversation_id ON token_gates(conversation_id);
CREATE INDEX idx_token_gates_created_at ON token_gates(created_at DESC);

-- Add comment for documentation
COMMENT ON TABLE token_gates IS 'Token gating requirements for group conversations';
COMMENT ON COLUMN token_gates.token_address IS 'ERC-20 token address on Base network, NULL for native ETH';
COMMENT ON COLUMN token_gates.min_amount IS 'Minimum token balance required (stored as string to preserve decimals)';
COMMENT ON COLUMN token_gates.operator IS 'Logical operator: AND (all requirements) or OR (any requirement)';
