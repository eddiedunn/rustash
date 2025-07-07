-- Drop old FTS triggers and table
DROP TRIGGER IF EXISTS snippets_fts_delete;
DROP TRIGGER IF EXISTS snippets_fts_update;
DROP TRIGGER IF EXISTS snippets_fts_insert;

-- Rename old tables
ALTER TABLE snippets RENAME TO snippets_old;
ALTER TABLE snippets_fts RENAME TO snippets_fts_old;

-- Create new tables with UUID support
CREATE TABLE snippets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '[]',
    embedding BLOB,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create FTS virtual table for full-text search
CREATE VIRTUAL TABLE snippets_fts USING fts5(
    title,
    content,
    tags,
    content_rowid='id'
);

-- Copy data from old tables, generating UUIDs for existing records
INSERT INTO snippets (id, uuid, title, content, tags, embedding, created_at, updated_at)
SELECT 
    id, 
    LOWER(HEX(RANDOMBLOB(4))) || '-' || LOWER(HEX(RANDOMBLOB(2))) || '-4' || 
    SUBSTR(LOWER(HEX(RANDOMBLOB(2))), 2) || '-' || 
    SUBSTR('89ab', ABS(RANDOM()) % 4 + 1, 1) || 
    SUBSTR(LOWER(HEX(RANDOMBLOB(2))), 2) || '-' || 
    LOWER(HEX(RANDOMBLOB(6))),
    title, 
    content, 
    tags, 
    embedding, 
    created_at, 
    updated_at 
FROM snippets_old;

-- Recreate FTS triggers
CREATE TRIGGER snippets_fts_insert AFTER INSERT ON snippets
BEGIN
    INSERT INTO snippets_fts (rowid, title, content, tags)
    VALUES (new.id, new.title, new.content, new.tags);
END;

CREATE TRIGGER snippets_fts_delete AFTER DELETE ON snippets
BEGIN
    DELETE FROM snippets_fts WHERE rowid = old.id;
END;

CREATE TRIGGER snippets_fts_update AFTER UPDATE ON snippets
BEGIN
    DELETE FROM snippets_fts WHERE rowid = old.id;
    INSERT INTO snippets_fts (rowid, title, content, tags)
    VALUES (new.id, new.title, new.content, new.tags);
END;

-- Drop old tables
DROP TABLE IF EXISTS snippets_old;
DROP TABLE IF EXISTS snippets_fts_old;
