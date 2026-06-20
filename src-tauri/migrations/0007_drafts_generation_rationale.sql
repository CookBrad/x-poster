-- Add optional generation_rationale for storing Grok's self-reported "added insight" / originality angle
-- from draft generation (used to help user during mandatory review/edit in DraftEditModal).
-- Nullable so legacy + manually created drafts are unaffected.

ALTER TABLE drafts ADD COLUMN generation_rationale TEXT;
