-- Drop triggers first
DROP TRIGGER IF EXISTS snippets_au;
DROP TRIGGER IF EXISTS snippets_ad;
DROP TRIGGER IF EXISTS snippets_ai;

-- Drop FTS table and its components
DROP TABLE IF EXISTS snippets_fts_idx;
DROP TABLE IF EXISTS snippets_fts_docsize;
DROP TABLE IF EXISTS snippets_fts_data;
DROP TABLE IF EXISTS snippets_fts_content;
DROP TABLE IF EXISTS snippets_fts_config;
DROP TABLE IF EXISTS snippets_fts;

-- Drop indexes
DROP INDEX IF EXISTS idx_snippets_updated_at;
DROP INDEX IF EXISTS idx_snippets_created_at;
DROP INDEX IF EXISTS idx_snippets_title;

-- Drop main table
DROP TABLE IF EXISTS snippets;