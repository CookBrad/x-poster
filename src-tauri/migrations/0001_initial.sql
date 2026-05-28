-- Initial schema for x-poster
-- Stores drafts in the queue and basic history

CREATE TABLE IF NOT EXISTS drafts (
    id TEXT PRIMARY KEY,
    text TEXT NOT NULL,
    sources_json TEXT NOT NULL,           -- JSON array of sources
    image_url TEXT,                       -- optional stock or generated image
    status TEXT NOT NULL DEFAULT 'pending', -- pending | posted | skipped
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    posted_at TEXT,
    x_post_id TEXT                        -- Twitter/X post ID after successful post
);

CREATE INDEX IF NOT EXISTS idx_drafts_status ON drafts(status);
CREATE INDEX IF NOT EXISTS idx_drafts_created_at ON drafts(created_at);

-- Simple history / audit table (can evolve later)
CREATE TABLE IF NOT EXISTS post_history (
    id TEXT PRIMARY KEY,
    draft_id TEXT,
    text TEXT NOT NULL,
    x_post_id TEXT NOT NULL,
    posted_at TEXT NOT NULL,
    topic TEXT
);

CREATE INDEX IF NOT EXISTS idx_history_posted_at ON post_history(posted_at);