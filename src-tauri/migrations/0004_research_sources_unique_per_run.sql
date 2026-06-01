-- Prevent duplicate sources within the same research run.
-- We use (run_id, original_id) as the uniqueness key.

CREATE UNIQUE INDEX IF NOT EXISTS idx_research_sources_run_original 
ON research_sources (run_id, original_id);