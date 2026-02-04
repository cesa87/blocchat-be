-- Create enum for transaction status
CREATE TYPE transaction_status AS ENUM ('pending', 'confirmed', 'failed');

-- Create transactions table
CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY,
    tx_hash VARCHAR(66) NOT NULL UNIQUE,
    from_address VARCHAR(42) NOT NULL,
    to_address VARCHAR(42) NOT NULL,
    amount TEXT NOT NULL,
    token_address VARCHAR(42),
    chain_id INTEGER NOT NULL,
    conversation_id TEXT NOT NULL,
    message_id TEXT,
    status transaction_status NOT NULL DEFAULT 'pending',
    block_number BIGINT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    confirmed_at TIMESTAMP WITH TIME ZONE
);

-- Create indexes
CREATE INDEX idx_transactions_tx_hash ON transactions(tx_hash);
CREATE INDEX idx_transactions_conversation_id ON transactions(conversation_id);
CREATE INDEX idx_transactions_from_address ON transactions(from_address);
CREATE INDEX idx_transactions_to_address ON transactions(to_address);
CREATE INDEX idx_transactions_status ON transactions(status);
CREATE INDEX idx_transactions_created_at ON transactions(created_at DESC);
