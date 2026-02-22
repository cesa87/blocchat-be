-- Public group registry for discoverable groups
CREATE TABLE IF NOT EXISTS public_groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT,
    image_url TEXT,
    owner_inbox_id TEXT NOT NULL,
    owner_wallet TEXT NOT NULL,
    is_public BOOLEAN NOT NULL DEFAULT true,
    member_count INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for search and lookups
CREATE INDEX idx_public_groups_name ON public_groups USING gin(to_tsvector('english', name));
CREATE INDEX idx_public_groups_owner ON public_groups(owner_inbox_id);
CREATE INDEX idx_public_groups_public ON public_groups(is_public) WHERE is_public = true;

-- Auto-update updated_at
CREATE TRIGGER update_public_groups_updated_at BEFORE UPDATE ON public_groups
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

COMMENT ON TABLE public_groups IS 'Registry of public/discoverable groups - metadata only, no messages';
