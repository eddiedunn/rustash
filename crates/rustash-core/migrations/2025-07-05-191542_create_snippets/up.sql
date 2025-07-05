-- Create snippets table
CREATE TABLE snippets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
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
CREATE VIRTUAL TABLE IF NOT EXISTS snippets_fts USING fts5(
    title,
    content,
    tags,
    content_rowid=id
);

-- Trigger to keep FTS table in sync
CREATE TRIGGER snippets_fts_insert AFTER INSERT ON snippets BEGIN
    INSERT INTO snippets_fts(rowid, title, content, tags)
    VALUES (new.id, new.title, new.content, new.tags);
END;

CREATE TRIGGER snippets_fts_update AFTER UPDATE ON snippets BEGIN
    UPDATE snippets_fts SET
        title = new.title,
        content = new.content,
        tags = new.tags
    WHERE rowid = new.id;
END;

CREATE TRIGGER snippets_fts_delete AFTER DELETE ON snippets BEGIN
    DELETE FROM snippets_fts WHERE rowid = old.id;
END;