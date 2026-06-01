-- Research runs and sources storage

CREATE TABLE IF NOT EXISTS research_runs (
    id TEXT PRIMARY KEY,
    run_at TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'manual'   -- 'manual' | 'scheduled' (for future use)
);

CREATE INDEX IF NOT EXISTS idx_research_runs_run_at ON research_runs(run_at DESC);

CREATE TABLE IF NOT EXISTS research_sources (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    url TEXT NOT NULL,
    published_at TEXT,
    source_name TEXT NOT NULL,
    source_type TEXT NOT NULL,              -- 'rss', 'x', 'x_grok', etc.
    retweet_count INTEGER,
    like_count INTEGER,
    reply_count INTEGER,
    quote_count INTEGER,
    FOREIGN KEY(run_id) REFERENCES research_runs(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_research_sources_run_id ON research_sources(run_id);