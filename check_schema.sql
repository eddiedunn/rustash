-- Check if the snippets table exists
SELECT name FROM sqlite_master WHERE type='table' AND name='snippets';

-- Check the schema of the snippets table if it exists
PRAGMA table_info(snippets);

-- Check if the snippets_old table exists
SELECT name FROM sqlite_master WHERE type='table' AND name='snippets_old';

-- Check if the __diesel_schema_migrations table exists
SELECT name FROM sqlite_master WHERE type='table' AND name='__diesel_schema_migrations';

-- List all tables in the database
SELECT name FROM sqlite_master WHERE type='table';

-- Check the content of the __diesel_schema_migrations table if it exists
SELECT * FROM __diesel_schema_migrations;
