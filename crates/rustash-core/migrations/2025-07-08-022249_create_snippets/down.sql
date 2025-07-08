-- Drop triggers first to avoid reference errors
DROP TRIGGER IF EXISTS snippets_ai;
DROP TRIGGER IF EXISTS snippets_au;
DROP TRIGGER IF EXISTS snippets_ad;

-- Drop FTS virtual table
DROP TABLE IF EXISTS snippets_fts;

-- Drop indexes
DROP INDEX IF EXISTS idx_snippets_title;
DROP INDEX IF EXISTS idx_snippets_created_at;
DROP INDEX IF EXISTS idx_snippets_updated_at;

-- Finally, drop the main table
DROP TABLE IF EXISTS snippets;
