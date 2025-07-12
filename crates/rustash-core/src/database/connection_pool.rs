//! A strongly-typed connection pool implementation for database backends.
//!
//! This module provides a type-safe wrapper around the connection pool that enforces
//! compile-time backend selection through feature flags.

use crate::error::{Error, Result};
use async_trait::async_trait;
use bb8::Pool;
use diesel_async::{
    pooled_connection::AsyncDieselConnectionManager, AsyncConnection, AsyncPgConnection,
    AsyncSqliteConnection,
};
use std::sync::Arc;

/// A trait representing a database backend connection.
#[async_trait]
pub trait DatabaseConnection: AsyncConnection + Send + 'static {
    /// The type of the connection manager for this backend.
    type Manager: for<'a> AsyncDieselConnectionManager<Self> + Send + 'static;

    /// The native connection type for this backend.
    type NativeConnection: AsyncConnection + Send + 'static;

    /// Create a new connection manager for the given database URL.
    fn create_manager(database_url: &str) -> Result<Self::Manager>;

    /// Run any necessary setup for the connection.
    async fn setup_connection(conn: &mut Self) -> Result<()>;
}

/// SQLite database connection implementation.
#[cfg(feature = "sqlite")]
#[async_trait]
impl DatabaseConnection for AsyncSqliteConnection {
    type Manager = AsyncDieselConnectionManager<Self>;
    type NativeConnection = AsyncSqliteConnection;

    fn create_manager(database_url: &str) -> Result<Self::Manager> {
        // Ensure the parent directory exists for SQLite file-based databases
        if !database_url.starts_with("file:")
            && !database_url.starts_with(":memory:")
            && !database_url.is_empty()
        {
            if let Some(parent) = std::path::Path::new(database_url).parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        Error::other(format!("Failed to create parent directory: {}", e))
                    })?;
                }
            }
        }

        Ok(AsyncDieselConnectionManager::<Self>::new(database_url))
    }

    async fn setup_connection(conn: &mut Self) -> Result<()> {
        use diesel_async::RunQueryDsl;
        use diesel::sql_query;

        // Enable foreign keys and WAL mode for SQLite
        sql_query("PRAGMA foreign_keys = ON")
            .execute(conn)
            .await
            .map_err(|e| Error::other(format!("Failed to enable foreign keys: {}", e)))?;

        sql_query("PRAGMA journal_mode = WAL")
            .execute(conn)
            .await
            .map_err(|e| Error::other(format!("Failed to enable WAL mode: {}", e)))?;

        Ok(())
    }
}

/// PostgreSQL database connection implementation.
#[cfg(feature = "postgres")]
#[async_trait]
impl DatabaseConnection for AsyncPgConnection {
    type Manager = AsyncDieselConnectionManager<Self>;
    type NativeConnection = AsyncPgConnection;

    fn create_manager(database_url: &str) -> Result<Self::Manager> {
        Ok(AsyncDieselConnectionManager::<Self>::new(database_url))
    }

    async fn setup_connection(&mut self) -> Result<()> {
        // PostgreSQL doesn't need any special setup by default
        Ok(())
    }
}

/// A strongly-typed connection pool for a specific database backend.
pub struct DbConnectionPool<C: DatabaseConnection> {
    inner: Arc<Pool<C::Manager>>,
}

impl<C> Clone for DbConnectionPool<C>
where
    C: DatabaseConnection,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<C> std::fmt::Debug for DbConnectionPool<C>
where
    C: DatabaseConnection,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbConnectionPool").finish()
    }
}

impl<C> DbConnectionPool<C>
where
    C: DatabaseConnection,
{
    /// Create a new connection pool for the given database URL.
    pub async fn new(database_url: &str) -> Result<Self> {
        let manager = C::create_manager(database_url)?;
        let pool = Pool::builder()
            .build(manager)
            .await
            .map_err(|e| Error::other(format!("Failed to create connection pool: {}", e)))?;

        // Test the connection
        let mut conn = pool
            .get()
            .await
            .map_err(|e| Error::other(format!("Failed to get connection from pool: {}", e)))?;

        // Run any necessary setup
        C::setup_connection(&mut *conn).await?;

        Ok(Self {
            inner: Arc::new(pool),
        })
    }

    /// Get a connection from the pool.
    pub async fn get_connection(
        &self,
    ) -> Result<bb8::PooledConnection<'_, C::Manager>> {
        self.inner
            .get()
            .await
            .map_err(|e| Error::other(format!("Failed to get connection from pool: {}", e)))
    }

    /// Get a reference to the inner pool.
    pub fn inner(&self) -> &Pool<C::Manager> {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::sql_query;

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_sqlite_pool() -> Result<()> {
        let pool = DbConnectionPool::<AsyncSqliteConnection>::new(":memory:").await?;
        let mut conn = pool.get_connection().await?;
        
        // Test that we can execute a simple query
        let result: i32 = sql_query("SELECT 1")
            .get_result(&mut *conn)
            .await
            .map_err(|e| Error::other(format!("Query failed: {}", e)))?;
            
        assert_eq!(result, 1);
        Ok(())
    }

    #[cfg(feature = "postgres")]
    #[tokio::test]
    async fn test_postgres_pool() -> Result<()> {
        let database_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/test".to_string()
        });
        
        let pool = DbConnectionPool::<AsyncPgConnection>::new(&database_url).await?;
        let mut conn = pool.get_connection().await?;
        
        // Test that we can execute a simple query
        let result: i32 = sql_query("SELECT 1")
            .get_result(&mut *conn)
            .await
            .map_err(|e| Error::other(format!("Query failed: {}", e)))?;
            
        assert_eq!(result, 1);
        Ok(())
    }
}
