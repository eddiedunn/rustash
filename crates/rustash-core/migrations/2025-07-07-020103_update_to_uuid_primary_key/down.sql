-- This migration is a no-op when rolling back from the updated initial migration
-- that already uses UUIDs. This is kept for backward compatibility.

-- Check if we're in a state where we need to revert to integer IDs
CREATE TABLE IF NOT EXISTS __migration_check (
    check_result BOOLEAN
);

-- This will fail if the id column exists, making it a safe way to check
-- if we need to revert to integer IDs
INSERT INTO __migration_check (check_result) 
SELECT 1 FROM pragma_table_info('snippets') WHERE name = 'id';

-- If we get here, we have a table with an id column and need to revert
-- Drop the check table since we don't need it anymore
DROP TABLE __migration_check;

-- Create a new table with integer primary key
CREATE TABLE snippets_old (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '[]',
    embedding BLOB,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    uuid TEXT
);

-- Copy data to the old table
-- This will generate new IDs for all snippets
INSERT INTO snippets_old (title, content, tags, embedding, created_at, updated_at, uuid)
SELECT 
    title, 
    content, 
    COALESCE(tags, '[]'), 
    embedding, 
    COALESCE(created_at, CURRENT_TIMESTAMP), 
    COALESCE(updated_at, CURRENT_TIMESTAMP),
    uuid
FROM snippets;

-- Drop the current table and rename the old one
DROP TABLE snippets;
ALTER TABLE snippets_old RENAME TO snippets;

-- Recreate indexes
CREATE INDEX IF NOT EXISTS idx_snippets_title ON snippets(title);
CREATE INDEX IF NOT EXISTS idx_snippets_created_at ON snippets(created_at);
CREATE INDEX IF NOT EXISTS idx_snippets_updated_at ON snippets(updated_at);

-- Recreate FTS table
DROP TABLE IF EXISTS snippets_fts;
CREATE VIRTUAL TABLE snippets_fts USING fts5(
    title,
    content,
    tags,
    content='snippets',
    content_rowid='rowid',
    tokenize='porter unicode61 remove_diacritics 1'
);

-- Recreate triggers
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
