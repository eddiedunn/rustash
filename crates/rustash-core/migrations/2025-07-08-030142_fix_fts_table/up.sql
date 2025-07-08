-- Drop existing FTS table and triggers
DROP TRIGGER IF EXISTS snippets_ai;
DROP TRIGGER IF EXISTS snippets_au;
DROP TRIGGER IF EXISTS snippets_ad;
DROP TABLE IF EXISTS snippets_fts;

-- Recreate FTS virtual table for full-text search without the problematic column
CREATE VIRTUAL TABLE snippets_fts USING fts5(
    title,
    content,
    tags,
    content='snippets',
    content_rowid='rowid',
    tokenize='porter unicode61 remove_diacritics 1',
    prefix='2,3,4,5,6,7',
    columnsize=0,
    detail=full
);

-- Recreate triggers to keep FTS table in sync with snippets table
CREATE TRIGGER snippets_ai AFTER INSERT ON snippets
BEGIN
    INSERT INTO snippets_fts (rowid, title, content, tags)
    VALUES (new.rowid, new.title, new.content, new.tags);
END;

CREATE TRIGGER snippets_au AFTER UPDATE ON snippets
BEGIN
    UPDATE snippets_fts
    SET title = new.title,
        content = new.content,
        tags = new.tags
    WHERE rowid = old.rowid;
END;

CREATE TRIGGER snippets_ad AFTER DELETE ON snippets
BEGIN
    DELETE FROM snippets_fts WHERE rowid = old.rowid;
END;

-- Rebuild the FTS index
INSERT INTO snippets_fts (rowid, title, content, tags)
SELECT rowid, title, content, tags FROM snippets;
