//! Database connection and pooling functionality for Rustash.
//!
//! This module provides database connection pooling and management for different
//! database backends (SQLite and PostgreSQL) with compile-time backend selection.

#![allow(dead_code)] // Temporary until all code is migrated

pub mod connection_pool;

// Re-export commonly used types
#[cfg(feature = "postgres")]
pub use diesel_async::AsyncPgConnection;
#[cfg(feature = "sqlite")]
pub use diesel_async::AsyncSqliteConnection;
pub use diesel_async::AsyncConnection;

// Re-export connection pool types
pub use connection_pool::DbConnectionPool;

/// Database connection types based on the enabled feature
#[cfg(feature = "sqlite")]
pub type Connection = AsyncSqliteConnection;

#[cfg(feature = "postgres")]
pub type Connection = AsyncPgConnection;

/// A type alias for the database connection pool based on the enabled feature
#[cfg(feature = "sqlite")]
pub type DbPool = DbConnectionPool<AsyncSqliteConnection>;

#[cfg(feature = "postgres")]
pub type DbPool = DbConnectionPool<AsyncPgConnection>;

/// Create a new database connection pool for the given URL.
///
/// The URL format depends on the database backend:
/// - SQLite: `file:path/to/database.db` or `:memory:` for in-memory database
/// - PostgreSQL: `postgres://user:password@localhost:5432/database`
pub async fn create_connection_pool(database_url: &str) -> Result<DbPool, crate::error::Error> {
    #[cfg(feature = "sqlite")]
    {
        DbConnectionPool::<AsyncSqliteConnection>::new(database_url).await
    }
    
    #[cfg(feature = "postgres")]
    {
        DbConnectionPool::<AsyncPgConnection>::new(database_url).await
    }
}

/// Create a test database connection pool for integration tests.
pub async fn create_test_pool() -> Result<DbPool, crate::error::Error> {
    #[cfg(feature = "sqlite")]
    {
        // Use an in-memory database for tests
        DbConnectionPool::<AsyncSqliteConnection>::new(":memory:").await
    }
    
    #[cfg(feature = "postgres")]
    {
        use std::env;
        
        let database_url = env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/test".to_string());
            
        DbConnectionPool::<AsyncPgConnection>::new(&database_url).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::sql_query;

    #[tokio::test]
    async fn test_connection_pool() -> Result<()> {
        let pool = create_test_pool().await?;
        let mut conn = pool.get_connection().await?;
        
        // Test that we can execute a simple query
        let result: i32 = sql_query("SELECT 1")
            .get_result(&mut *conn)
            .await
            .map_err(|e| crate::error::Error::other(format!("Query failed: {}", e)))?;
            
        assert_eq!(result, 1);
        Ok(())
    }
}
