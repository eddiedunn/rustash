-- This migration is no longer needed as the initial migration now uses UUIDs
-- from the start. This is kept as a no-op for backward compatibility.

-- Check if we need to migrate from an old schema
CREATE TABLE IF NOT EXISTS __migration_check (
    check_result BOOLEAN
);

-- This will fail if the uuid column doesn't exist, making it a safe way to check
-- if we need to migrate from an old schema
INSERT INTO __migration_check (check_result) 
SELECT 1 FROM pragma_table_info('snippets') WHERE name = 'id';

-- If we get here, we have an old schema and need to migrate
-- Drop the check table since we don't need it anymore
DROP TABLE __migration_check;

-- Add UUID column if it doesn't exist
ALTER TABLE snippets ADD COLUMN uuid TEXT;

-- Generate UUIDs for existing rows
UPDATE snippets SET uuid = lower(hex(randomblob(16))) WHERE uuid IS NULL;

-- Make UUID column NOT NULL
CREATE TABLE snippets_new (
    uuid TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '[]',
    embedding BLOB,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Copy data to new table
INSERT INTO snippets_new (uuid, title, content, tags, embedding, created_at, updated_at)
SELECT 
    COALESCE(uuid, lower(hex(randomblob(16)))),
    title, 
    content, 
    COALESCE(tags, '[]'),
    embedding, 
    COALESCE(created_at, CURRENT_TIMESTAMP),
    COALESCE(updated_at, CURRENT_TIMESTAMP)
FROM snippets;

-- Drop old table and rename new one
DROP TABLE snippets;
ALTER TABLE snippets_new RENAME TO snippets;

-- Recreate indexes
CREATE INDEX IF NOT EXISTS idx_snippets_title ON snippets(title);
CREATE INDEX IF NOT EXISTS idx_snippets_created_at ON snippets(created_at);
CREATE INDEX IF NOT EXISTS idx_snippets_updated_at ON snippets(updated_at);

-- Recreate FTS table if it doesn't exist
CREATE TABLE IF NOT EXISTS snippets_fts (
    title,
    content,
    tags,
    content='snippets',
    content_rowid='rowid',
    tokenize='porter unicode61 remove_diacritics 1'
);

-- Recreate triggers if they don't exist
CREATE TRIGGER IF NOT EXISTS snippets_ai AFTER INSERT ON snippets 
WHEN new.title IS NOT NULL
BEGIN
    INSERT INTO snippets_fts (rowid, title, content, tags)
    VALUES (new.rowid, new.title, new.content, COALESCE(new.tags, '[]'));
END;

CREATE TRIGGER IF NOT EXISTS snippets_ad AFTER DELETE ON snippets 
BEGIN
    INSERT INTO snippets_fts (snippets_fts, rowid, title, content, tags)
    VALUES ('delete', old.rowid, old.title, old.content, COALESCE(old.tags, '[]'));
END;

CREATE TRIGGER IF NOT EXISTS snippets_au AFTER UPDATE ON snippets 
WHEN new.title IS NOT NULL
BEGIN
    INSERT INTO snippets_fts (snippets_fts, rowid, title, content, tags)
    VALUES ('delete', old.rowid, old.title, old.content, COALESCE(old.tags, '[]'));
    
    INSERT INTO snippets_fts (rowid, title, content, tags)
    VALUES (new.rowid, new.title, new.content, COALESCE(new.tags, '[]'));
END;
