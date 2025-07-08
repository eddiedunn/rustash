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
            // Set up PostgreSQL backend
            let manager = bb8_postgres::PostgresConnectionManager::new_from_stringlike(
                database_url,
                tokio_postgres::NoTls
            ).map_err(|e| crate::error::Error::other(format!("Failed to create PostgreSQL connection manager: {}", e)))?;
            
            let pool = bb8::Pool::builder()
                .build(manager)
                .await
                .map_err(|e| crate::error::Error::other(format!("Failed to create PostgreSQL connection pool: {}", e)))?;
            
            // Initialize the database schema if needed
            let mut conn = pool.get().await
                .map_err(|e| crate::error::Error::other(format!("Failed to get connection from pool: {}", e)))?;
                
            conn.execute(
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
                "#,
                &[],
            ).await.map_err(|e| crate::error::Error::other(format!("Failed to create snippets table: {}", e)))?;
            
            // Create indexes
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_snippets_uuid ON snippets(uuid)",
                &[],
            ).await.map_err(|e| crate::error::Error::other(format!("Failed to create UUID index: {}", e)))?;
            
            // Create GIN index for tags (array operations)
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_snippets_created_at ON snippets(created_at)",
                &[],
            ).await.map_err(|e| crate::error::Error::other(format!("Failed to create created_at index: {}", e)))?;
            
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_snippets_updated_at ON snippets(updated_at)",
                &[],
            ).await.map_err(|e| crate::error::Error::other(format!("Failed to create updated_at index: {}", e)))?;
            
            // Set up full-text search if not exists
            // Note: PostgreSQL uses tsvector/tsquery for full-text search, not FTS5
            
            Ok(Box::new(PostgresBackend::new(pool)) as Box<dyn StorageBackend>)
        }
    } else if database_url.starts_with("sqlite") {
        #[cfg(not(feature = "sqlite"))]
        return Err(crate::error::Error::other("SQLite support not enabled. Enable with 'sqlite' feature."));
        
        #[cfg(feature = "sqlite")]
        {
            use diesel::{
                r2d2::{ConnectionManager, Pool},
                SqliteConnection,
            };
            use std::path::Path;
            use std::sync::Arc;

            // Clone the database URL for the async block
            let database_url = database_url.to_string();
            
            // Wrap the entire SQLite setup in spawn_blocking since it's I/O bound
            let backend = tokio::task::spawn_blocking(move || -> Result<Box<dyn StorageBackend>> {
                // Ensure the database directory exists
                if let Some(parent) = Path::new(database_url.trim_start_matches("sqlite://")).parent() {
                    if !parent.exists() {
                        std::fs::create_dir_all(parent).map_err(|e| 
                            crate::error::Error::other(format!("Failed to create database directory: {}", e))
                        )?;
                    }
                }

                // Create connection manager and pool
                let manager = ConnectionManager::<SqliteConnection>::new(&database_url);
                let pool = Pool::builder()
                    .build(manager)
                    .map_err(|e| crate::error::Error::other(format!("Failed to create SQLite connection pool: {}", e)))?;

                // Run migrations in the blocking task
                let conn = &mut pool.get()
                    .map_err(|e| crate::error::Error::other(format!("Failed to get connection from pool: {}", e)))?;
                    
                // Use the migration API correctly
                use diesel_migrations::{FileBasedMigrations, MigrationHarness};
                
                // Define the migrations directory
                let migrations_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
                let migrations = FileBasedMigrations::from_path(&migrations_dir)
                    .map_err(|e| crate::error::Error::other(format!("Failed to load migrations: {}", e)))?;
                    
                // Run pending migrations
                conn.run_pending_migrations(migrations)
                    .map_err(|e| crate::error::Error::other(format!("Failed to run migrations: {}", e)))?;

                Ok(Box::new(SqliteBackend::new(pool)) as Box<dyn StorageBackend>)
            })
            .await
            .map_err(|e| crate::error::Error::other(format!("Blocking task panicked: {}", e)))??;
            
            Ok(backend)
        }
    } else {
        // Fall back to in-memory backend
        Ok(Box::new(InMemoryBackend::default()))
    }
}