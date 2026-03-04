-- Add Basename fields to user_profiles
-- basename: the resolved .base.eth name (e.g. "alice.base.eth"), stored without prefix
-- basename_discoverable: user opt-in to make their basename searchable

ALTER TABLE user_profiles
    ADD COLUMN IF NOT EXISTS basename VARCHAR(100),
    ADD COLUMN IF NOT EXISTS basename_discoverable BOOLEAN NOT NULL DEFAULT false;

-- Unique index: only one profile per basename (partial — allows multiple NULLs)
CREATE UNIQUE INDEX IF NOT EXISTS idx_user_profiles_basename
    ON user_profiles (LOWER(basename))
    WHERE basename IS NOT NULL;

-- Index to efficiently find discoverable basenames during search
CREATE INDEX IF NOT EXISTS idx_user_profiles_basename_discoverable
    ON user_profiles (LOWER(basename))
    WHERE basename IS NOT NULL AND basename_discoverable = true;

COMMENT ON COLUMN user_profiles.basename IS '.base.eth name registered/detected for this wallet (e.g. alice.base.eth)';
COMMENT ON COLUMN user_profiles.basename_discoverable IS 'When true, users can find this profile by searching their basename';
