-- Add original_id column so we can safely store the same source across multiple research runs.
-- We will now use a fresh UUID as the primary key (id) for each row in research_sources.

ALTER TABLE research_sources ADD COLUMN original_id TEXT;

-- Optional: create an index for fast lookup of a specific source across runs
CREATE INDEX IF NOT EXISTS idx_research_sources_original_id ON research_sources(original_id);