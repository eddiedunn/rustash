-- Create snippets table with UUID as primary key
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

-- Full-text search index for content
CREATE VIRTUAL TABLE snippets_fts USING fts5(
    title,
    content,
    tags,
    content='snippets',
    content_rowid='rowid',
    tokenize='porter unicode61 remove_diacritics 1'
);

-- Triggers to keep FTS table in sync
CREATE TRIGGER snippets_ai AFTER INSERT ON snippets BEGIN
    INSERT INTO snippets_fts (rowid, title, content, tags)
    VALUES (new.rowid, new.title, new.content, new.tags);
END;

CREATE TRIGGER snippets_ad AFTER DELETE ON snippets BEGIN
    INSERT INTO snippets_fts (snippets_fts, rowid, title, content, tags)
    VALUES ('delete', old.rowid, old.title, old.content, old.tags);
END;

CREATE TRIGGER snippets_au AFTER UPDATE ON snippets BEGIN
    INSERT INTO snippets_fts (snippets_fts, rowid, title, content, tags)
    VALUES ('delete', old.rowid, old.title, old.content, old.tags);
    
    INSERT INTO snippets_fts (rowid, title, content, tags)
    VALUES (new.rowid, new.title, new.content, new.tags);
END;