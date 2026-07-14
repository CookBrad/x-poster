-- Optional X tweet id this draft is meant to reply to (Generate Reply flow).
-- When set, post_draft_to_x posts as an in_reply_to reply via API v2.
-- Nullable so existing standalone drafts are unaffected.

ALTER TABLE drafts ADD COLUMN in_reply_to_tweet_id TEXT;
