-- Drop triggers first
DROP TRIGGER IF EXISTS snippets_fts_delete;
DROP TRIGGER IF EXISTS snippets_fts_update;
DROP TRIGGER IF EXISTS snippets_fts_insert;

-- Drop FTS table
DROP TABLE IF EXISTS snippets_fts;

-- Drop indexes
DROP INDEX IF EXISTS idx_snippets_updated_at;
DROP INDEX IF EXISTS idx_snippets_created_at;
DROP INDEX IF EXISTS idx_snippets_title;

-- Drop main table
DROP TABLE IF EXISTS snippets;