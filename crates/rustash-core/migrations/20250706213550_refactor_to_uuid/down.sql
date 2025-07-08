-- Migration to revert the UUID refactoring
-- This is a no-op since we're not actually changing the schema here
-- The actual schema changes were done in the initial migration

-- No-op since we don't want to drop the UUID columns
SELECT 1;
