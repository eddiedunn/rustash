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
pub use database::establish_connection;
pub use error::{Error, Result};
pub use memory::MemoryItem;
pub use models::{DbSnippet, NewDbSnippet, Snippet, SnippetWithTags};
pub use storage::{
    postgres::PostgresBackend,
    sqlite::SqliteBackend,
    InMemoryBackend,
    StorageBackend,
};
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
        // Set up PostgreSQL backend
        let manager = bb8_postgres::PostgresConnectionManager::new_from_stringlike(database_url, tokio_postgres::NoTls)?;
        let pool = bb8::Pool::builder().build(manager).await?;
        
        // Initialize the database schema if needed
        let mut conn = pool.get().await?;
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS snippets (
                id SERIAL PRIMARY KEY,
                uuid UUID NOT NULL UNIQUE,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                tags JSONB NOT NULL DEFAULT '[]'::jsonb,
                embedding VECTOR(1536),
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
            &[],
        ).await?;
        
        // Create indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_snippets_uuid ON snippets(uuid)",
            &[],
        ).await?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_snippets_tags ON snippets USING GIN(tags)",
            &[],
        ).await?;
        
        Ok(Box::new(PostgresBackend::new(pool)))
    } else if database_url.starts_with("sqlite") {
        // Set up SQLite backend
        use diesel::{
            r2d2::{ConnectionManager, Pool},
            sqlite::SqliteConnection,
        };
        
        let manager = ConnectionManager::<SqliteConnection>::new(database_url);
        let pool = Pool::builder().build(manager)?;
        
        // Run migrations
        let mut conn = pool.get()?;
        diesel_migrations::run_pending_migrations(&mut *conn)?;
        
        Ok(Box::new(SqliteBackend::new(pool)))
    } else {
        // Fall back to in-memory backend
        Ok(Box::new(InMemoryBackend::default()))
    }
}