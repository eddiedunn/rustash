-- This is a simplified version of the vector support migration that works with standard SQLite
-- without requiring the VSS extension.

-- For SQLite: Create a regular table to store vector data
-- This table will store the same data but without VSS-specific features
CREATE TABLE IF NOT EXISTS vss_snippets (
    rowid INTEGER PRIMARY KEY,
    embedding BLOB  -- Store vector data as BLOB for compatibility
);

-- For PostgreSQL: Initialize the graph for AGE.
-- This is a good place to ensure the graph exists.
-- This statement will fail harmlessly on SQLite and be ignored.
SELECT create_graph('rustash_graph');
