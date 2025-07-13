-- For SQLite: Create a virtual table for VSS (Vector Similarity Search)
-- This table will be automatically populated with data from the 'snippets' table.
-- We are assuming an embedding dimension of 384.
-- This statement will be ignored by PostgreSQL.
CREATE VIRTUAL TABLE IF NOT EXISTS vss_snippets USING vss0(
    embedding(384)
);

-- For PostgreSQL: Initialize the graph for AGE.
-- This is a good place to ensure the graph exists.
-- This statement will fail harmlessly on SQLite and be ignored.
SELECT create_graph('rustash_graph');
