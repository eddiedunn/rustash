//! Database connection and pooling functionality for Rustash.
//!
//! This module provides database connection pooling and management for different
//! database backends (SQLite and PostgreSQL) with compile-time backend selection.

#![allow(dead_code)] // Temporary until all code is migrated

pub mod connection_pool;

// Re-export commonly used types
use async_trait::async_trait;
use diesel_async::{
    pooled_connection::bb8::PooledConnection,
    AsyncConnection,
    RunQueryDsl,
};

#[cfg(feature = "postgres")]
pub use diesel_async::{
    pg::PgRow,
    AsyncPgConnection,
};

#[cfg(feature = "sqlite")]
pub use diesel_async::AsyncSqliteConnection;

// Re-export connection pool types
pub use connection_pool::{
    DatabaseConnection, 
    DbConnection,
    AnyConnectionPool,
};

#[cfg(feature = "sqlite")]
pub use connection_pool::SqliteConnection;

#[cfg(feature = "postgres")]
pub use connection_pool::PostgresConnection;

/// Alias for the database pool type.
pub type DbPool = DbConnection;

/// Create a new database connection based on the URL scheme.
pub async fn create_connection(database_url: &str) -> crate::error::Result<DbPool> {
    if database_url.starts_with("postgres") {
        #[cfg(feature = "postgres")]
        {
            DbConnection::postgres(database_url).await
        }
        #[cfg(not(feature = "postgres"))]
        {
            Err(crate::error::Error::other(
                "PostgreSQL support is not enabled. Enable the 'postgres' feature.",
            ))
        }
    } else {
        #[cfg(feature = "sqlite")]
        {
            DbConnection::sqlite(database_url).await
        }
        #[cfg(not(feature = "sqlite"))]
        {
            Err(crate::error::Error::other(
                "SQLite support is not enabled. Enable the 'sqlite' feature.",
            ))
        }
    }
}

/// Create a new database connection pool for the given URL.
///
/// The URL format depends on the database backend:
/// - SQLite: `file:path/to/database.db` or `:memory:` for in-memory database
/// - PostgreSQL: `postgres://user:password@localhost:5432/database`
pub async fn create_connection_pool(database_url: &str) -> Result<DbPool, crate::error::Error> {
    #[cfg(feature = "sqlite")]
    {
        DbConnection::sqlite(database_url).await
    }

    #[cfg(feature = "postgres")]
    {
        DbConnection::postgres(database_url).await
    }
}

/// Backwards-compatible wrapper around `create_connection`.
pub async fn create_pool(database_url: &str) -> Result<DbPool, crate::error::Error> {
    create_connection(database_url).await
}

/// Run database migrations for the given connection pool.
pub async fn run_migrations(pool: &DbPool) -> Result<(), crate::error::Error> {
    use diesel_migrations::{embed_migrations, MigrationHarness};
    
    let migrations = embed_migrations!("migrations");
    
    let mut conn = pool.get_connection().await?;
    conn.run_pending_migrations(migrations)
        .await
        .map_err(|e| crate::error::Error::other(format!("Migration failed: {}", e)))?;
        
    Ok(())
}

/// Create a test database connection pool for integration tests.
pub async fn create_test_pool() -> Result<DbPool, crate::error::Error> {
    #[cfg(feature = "sqlite")]
    let pool = DbConnection::sqlite(":memory:").await?;

    #[cfg(feature = "postgres")]
    let pool = DbConnection::postgres(
        &std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost/rustash_test".to_string()
        }),
    )
    .await?;

    Ok(pool)
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
