//! Rustash Core Library

pub mod config;
pub mod database;
pub mod error;
pub mod graph;
pub mod memory;
pub mod models;
pub mod rag;
pub mod schema;
pub mod snippet;
pub mod stash;
pub mod storage;

#[cfg(feature = "vector-search")]
pub mod search;

// Re-export commonly used types
pub use error::{Error, Result};
pub use memory::MemoryItem;
pub use models::{NewDbSnippet, Snippet, SnippetWithTags};
pub use stash::{ServiceType, Stash, StashConfig};
pub use storage::{InMemoryBackend, StorageBackend};

#[cfg(feature = "postgres")]
pub use storage::postgres::PostgresBackend;

#[cfg(feature = "sqlite")]
pub use storage::sqlite::SqliteBackend;

pub use snippet::{expand_placeholders, validate_snippet_content, SnippetService};

#[cfg(feature = "vector-search")]
pub use search::search_similar_snippets;

/// Create a new storage backend dynamically based on the database URL.
pub async fn create_backend(database_url: &str) -> Result<Box<dyn StorageBackend>> {
    if database_url.starts_with("postgres") {
        #[cfg(not(feature = "postgres"))]
        return Err(crate::error::Error::other(
            "PostgreSQL support not enabled. Recompile with the 'postgres' feature.",
        ));

        #[cfg(feature = "postgres")]
        {
            let pool = crate::database::postgres_pool::create_pool(database_url).await?;
            Ok(Box::new(PostgresBackend::new(pool)))
        }
    } else if database_url.starts_with("sqlite") {
        #[cfg(not(feature = "sqlite"))]
        return Err(crate::error::Error::other(
            "SQLite support not enabled. Recompile with the 'sqlite' feature.",
        ));

        #[cfg(feature = "sqlite")]
        {
            let pool = crate::database::sqlite_pool::create_pool(database_url).await?;
            Ok(Box::new(SqliteBackend::new(pool)))
        }
    } else {
        Err(crate::error::Error::other(
            "Unsupported database URL scheme. Use 'sqlite://' or 'postgres://'.",
        ))
    }
}
