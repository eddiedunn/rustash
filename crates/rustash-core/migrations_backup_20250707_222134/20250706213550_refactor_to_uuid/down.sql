-- Drop triggers and tables
DROP TRIGGER IF EXISTS snippets_fts_delete;
DROP TRIGGER IF EXISTS snippets_fts_update;
DROP TRIGGER IF EXISTS snippets_fts_insert;

-- Rename current tables
ALTER TABLE snippets RENAME TO snippets_new;
ALTER TABLE snippets_fts RENAME TO snippets_fts_new;

-- Recreate original schema
CREATE TABLE snippets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '[]',
    embedding BLOB,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Copy data back, dropping the UUID column
INSERT INTO snippets (id, title, content, tags, embedding, created_at, updated_at)
SELECT id, title, content, tags, embedding, created_at, updated_at FROM snippets_new;

-- Recreate FTS virtual table
CREATE VIRTUAL TABLE snippets_fts USING fts5(
    title,
    content,
    tags,
    content_rowid='id'
);

-- Recreate triggers
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

-- Drop the new tables
DROP TABLE snippets_new;
DROP TABLE snippets_fts_new;
