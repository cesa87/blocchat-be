-- Create shops table
CREATE TABLE IF NOT EXISTS shops (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    owner_address VARCHAR(42) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create shop_items table
CREATE TABLE IF NOT EXISTS shop_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    shop_id UUID NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    price TEXT NOT NULL,
    token_address VARCHAR(42),
    token_symbol VARCHAR(20) NOT NULL,
    image_url TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create index on conversation_id for faster lookups
CREATE INDEX IF NOT EXISTS idx_shops_conversation_id ON shops(conversation_id);

-- Create index on shop_id for faster item lookups
CREATE INDEX IF NOT EXISTS idx_shop_items_shop_id ON shop_items(shop_id);
