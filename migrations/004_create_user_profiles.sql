-- Create user_profiles table for username system
CREATE TABLE IF NOT EXISTS user_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_address VARCHAR(42) NOT NULL UNIQUE,
    inbox_id TEXT NOT NULL UNIQUE,
    username VARCHAR(30) UNIQUE,
    display_name VARCHAR(50),
    avatar_url TEXT,
    bio TEXT,
    last_username_change TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create indexes for fast lookups
CREATE INDEX idx_user_profiles_wallet ON user_profiles(wallet_address);
CREATE INDEX idx_user_profiles_inbox ON user_profiles(inbox_id);
CREATE INDEX idx_user_profiles_username ON user_profiles(username) WHERE username IS NOT NULL;

-- Create a function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create trigger to auto-update updated_at
CREATE TRIGGER update_user_profiles_updated_at BEFORE UPDATE ON user_profiles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Add comments for documentation
COMMENT ON TABLE user_profiles IS 'User profile data for username system - NO MESSAGE CONTENT';
COMMENT ON COLUMN user_profiles.username IS 'Unique username, alphanumeric + underscore, 3-30 chars';
COMMENT ON COLUMN user_profiles.last_username_change IS 'Last time username was changed (30 day cooldown)';
