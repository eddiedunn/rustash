-- Create the snippets table with UUID as primary key
CREATE TABLE snippets (
    uuid TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '[]', -- JSON array of tags
    embedding BLOB, -- Vector embedding for similarity search
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for better performance
CREATE INDEX idx_snippets_title ON snippets(title);
CREATE INDEX idx_snippets_created_at ON snippets(created_at);
CREATE INDEX idx_snippets_updated_at ON snippets(updated_at);

-- Create FTS virtual table for full-text search
CREATE VIRTUAL TABLE snippets_fts USING fts5(
    title,
    content,
    tags,
    content='snippets',
    content_rowid='rowid',
    tokenize='porter unicode61 remove_diacritics 1',
    prefix='2,3,4,5,6,7',  -- Enable prefix search for 2-7 character prefixes
    columnsize=0,          -- Don't store the content (we have it in the main table)
    detail=full            -- Store all token positions for better search
);

-- Triggers to keep FTS table in sync with snippets table

-- After insert trigger
CREATE TRIGGER snippets_ai AFTER INSERT ON snippets
BEGIN
    INSERT INTO snippets_fts (rowid, title, content, tags)
    VALUES (new.rowid, new.title, new.content, new.tags);
END;

-- After update trigger
CREATE TRIGGER snippets_au AFTER UPDATE ON snippets
BEGIN
    UPDATE snippets_fts
    SET title = new.title,
        content = new.content,
        tags = new.tags
    WHERE rowid = old.rowid;
END;

-- After delete trigger
CREATE TRIGGER snippets_ad AFTER DELETE ON snippets
BEGIN
    DELETE FROM snippets_fts WHERE rowid = old.rowid;
END;

-- Insert a test snippet
INSERT INTO snippets (uuid, title, content, tags)
VALUES (
    '00000000-0000-0000-0000-000000000000',
    'Welcome to Rustash',
    'This is your first snippet. Edit or delete it, then start creating your own!',
    '["welcome", "getting-started"]'
);
