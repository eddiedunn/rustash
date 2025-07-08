-- This migration doesn't have a clean rollback since we're fixing a schema issue
-- The best we can do is drop the FTS table and triggers
DROP TRIGGER IF EXISTS snippets_ai;
DROP TRIGGER IF EXISTS snippets_au;
DROP TRIGGER IF EXISTS snippets_ad;
DROP TABLE IF EXISTS snippets_fts;

-- The original FTS table will be recreated by the original migration
-- when running migrations forward again
-- This file should undo anything in `up.sql`
