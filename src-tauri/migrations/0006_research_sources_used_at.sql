ALTER TABLE research_sources ADD COLUMN used_at TEXT;

CREATE INDEX IF NOT EXISTS idx_research_sources_used_at ON research_sources(used_at);