//! Rustash Core Library
//!
//! This crate provides the core functionality for the Rustash snippet manager,
//! including database operations, data models, and snippet management.

pub mod database;
pub mod error;
pub mod memory;
pub mod models;
pub mod schema;
pub mod snippet;
pub mod storage;

#[cfg(feature = "vector-search")]
pub mod search;

// Re-export commonly used types
pub use database::{
    create_connection_pool, create_test_pool, Connection, DbConnectionPool, DbPool
};
pub use error::{Error, Result};
pub use memory::MemoryItem;
pub use models::{DbSnippet, NewDbSnippet, Snippet, SnippetWithTags};
pub use storage::{
    InMemoryBackend,
    StorageBackend,
};

#[cfg(feature = "postgres")]
pub use storage::postgres::PostgresBackend;

#[cfg(feature = "sqlite")]
pub use storage::sqlite::SqliteBackend;
pub use uuid::Uuid;
pub use snippet::{
    add_snippet, delete_snippet, expand_placeholders, get_snippet_by_id, list_snippets,
    list_snippets_with_tags, search_snippets, update_snippet,
};

#[cfg(feature = "vector-search")]
pub use search::search_similar_snippets;

/// Create a new storage backend based on the provided database URL.
/// 
/// # Arguments
/// * `database_url` - The database URL to connect to (e.g., "sqlite:///path/to/db.sqlite" or "postgres://user:pass@localhost:5432/db")
/// 
/// # Returns
/// A `Result` containing a boxed `dyn StorageBackend` if successful, or an error.
pub async fn create_backend(database_url: &str) -> Result<Box<dyn StorageBackend>> {
    if database_url.starts_with("postgres") {
        #[cfg(not(feature = "postgres"))]
        return Err(crate::error::Error::other("PostgreSQL support not enabled. Enable with 'postgres' feature."));
        
        #[cfg(feature = "postgres")]
        {
            use diesel_async::RunQueryDsl;
            
            // Create a new database pool
            let pool = database::create_pool(database_url).await?;
            
            // Initialize the database schema if needed
            let mut conn = pool.get().await?;
            
            // Create the snippets table
            diesel::sql_query(
                r#"
                CREATE TABLE IF NOT EXISTS snippets (
                    uuid TEXT PRIMARY KEY NOT NULL,
                    title TEXT NOT NULL,
                    content TEXT NOT NULL,
                    tags TEXT NOT NULL DEFAULT '[]',
                    embedding BYTEA,
                    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
                )
                "#
            )
            .execute(&mut conn)
            .await
            .map_err(|e| crate::error::Error::other(format!("Failed to create snippets table: {}", e)))?;
            
            // Create indexes
            diesel::sql_query(
                "CREATE INDEX IF NOT EXISTS idx_snippets_uuid ON snippets(uuid)"
            )
            .execute(&mut conn)
            .await
            .map_err(|e| crate::error::Error::other(format!("Failed to create UUID index: {}", e)))?;
            
            diesel::sql_query(
                "CREATE INDEX IF NOT EXISTS idx_snippets_created_at ON snippets(created_at)"
            )
            .execute(&mut conn)
            .await
            .map_err(|e| crate::error::Error::other(format!("Failed to create created_at index: {}", e)))?;
            
            diesel::sql_query(
                "CREATE INDEX IF NOT EXISTS idx_snippets_updated_at ON snippets(updated_at)"
            )
            .execute(&mut conn)
            .await
            .map_err(|e| crate::error::Error::other(format!("Failed to create updated_at index: {}", e)))?;
            
            // Create a function to update the updated_at timestamp
            diesel::sql_query(
                r#"
                CREATE OR REPLACE FUNCTION update_updated_at_column()
                RETURNS TRIGGER AS $$
                BEGIN
                    NEW.updated_at = NOW();
                    RETURN NEW;
                END;
                $$ language 'plpgsql';
                "#
            )
            .execute(&mut conn)
            .await
            .map_err(|e| crate::error::Error::other(format!("Failed to create update_updated_at_column function: {}", e)))?;
            
            // Create a trigger to update the updated_at timestamp
            diesel::sql_query(
                r#"
                DROP TRIGGER IF EXISTS update_snippets_updated_at ON snippets;
                CREATE TRIGGER update_snippets_updated_at
                BEFORE UPDATE ON snippets
                FOR EACH ROW
                EXECUTE FUNCTION update_updated_at_column();
                "#
            )
            .execute(&mut conn)
            .await
            .map_err(|e| crate::error::Error::other(format!("Failed to create update trigger: {}", e)))?;
            
            Ok(Box::new(PostgresBackend::new(pool)) as Box<dyn StorageBackend>)
        }
    } else if database_url.starts_with("sqlite") {
        #[cfg(not(feature = "sqlite"))]
        return Err(crate::error::Error::other("SQLite support not enabled. Enable with 'sqlite' feature."));
        
        #[cfg(feature = "sqlite")]
        {
            use diesel::connection::SimpleConnection;
            
            // Create a new database pool
            let pool = database::DbPool::new(database_url).await?;
            
            // Initialize the database schema if needed
            let mut conn = pool.get_async().await?;
            
            conn.batch_execute(
                r#"
                CREATE TABLE IF NOT EXISTS snippets (
                    uuid TEXT PRIMARY KEY NOT NULL,
                    title TEXT NOT NULL,
                    content TEXT NOT NULL,
                    tags TEXT NOT NULL DEFAULT '[]',
                    embedding BLOB,
                    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                
                CREATE INDEX IF NOT EXISTS idx_snippets_uuid ON snippets(uuid);
                CREATE INDEX IF NOT EXISTS idx_snippets_created_at ON snippets(created_at);
                CREATE INDEX IF NOT EXISTS idx_snippets_updated_at ON snippets(updated_at);
                
                -- Create a trigger to update the updated_at timestamp
                DROP TRIGGER IF EXISTS update_snippets_updated_at;
                CREATE TRIGGER update_snippets_updated_at
                AFTER UPDATE ON snippets
                FOR EACH ROW
                BEGIN
                    UPDATE snippets SET updated_at = CURRENT_TIMESTAMP
                    WHERE uuid = NEW.uuid;
                END;
                "#,
            )?;
            
            Ok(Box::new(SqliteBackend::new(pool)) as Box<dyn StorageBackend>)
        }
    } else {
        // Fall back to in-memory backend
        Ok(Box::new(InMemoryBackend::default()))
    }
}