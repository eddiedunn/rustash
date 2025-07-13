-- This table is primarily for the SQLite backend to emulate graph relationships.
-- It can be ignored by the PostgreSQL/AGE backend.
CREATE TABLE IF NOT EXISTS relations (
    from_uuid TEXT NOT NULL,
    to_uuid TEXT NOT NULL,
    relation_type TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (from_uuid, to_uuid, relation_type)
);
