//! A connection pool implementation for database backends.
//!
//! This module provides a wrapper around the connection pool that works with
//! both SQLite and PostgreSQL backends.

use crate::error::{Error, Result};
use async_trait::async_trait;
use bb8::{Pool, PooledConnection};
use diesel_async::{
    pooled_connection::{AsyncDieselConnectionManager, bb8::PoolableConnection as _},
    AsyncConnection,
};
use std::sync::Arc;
use std::fmt::Debug;

/// A trait representing a database backend connection.
/// This is an object-safe trait that can be used with trait objects.
pub trait DatabaseConnection: Send + Sync + 'static {
    /// Get a connection from the pool.
    async fn get_connection(
        &self,
    ) -> Result<Box<dyn AsyncConnection + Send + 'static>>;

    /// Run any necessary setup for the connection.
    async fn setup_connection(&self, conn: &mut (dyn AsyncConnection + Send + 'static)) -> Result<()>;
}

/// A helper trait for creating database connections.
/// This is not object-safe but is used during initialization.
pub trait DatabaseConnectionFactory: Send + Sync + 'static {
    /// The type of the connection for this backend.
    type Connection: AsyncConnection + Send + 'static;

    /// Create a new connection manager for the given database URL.
    fn create_manager(database_url: &str) -> Result<AsyncDieselConnectionManager<Self::Connection>>
    where
        Self: Sized;

    /// Run any necessary setup for the connection.
    async fn setup_connection(conn: &mut Self::Connection) -> Result<()>;
}

/// A concrete implementation of DatabaseConnection for a specific backend.
pub struct DatabaseConnectionImpl<C>
where
    C: AsyncConnection + Send + 'static,
    AsyncDieselConnectionManager<C>: bb8::ManageConnection,
    Pool<AsyncDieselConnectionManager<C>>: Sync + Send,
{
    pool: Arc<Pool<AsyncDieselConnectionManager<C>>>,
}

impl<C> Debug for DatabaseConnectionImpl<C>
where
    C: AsyncConnection + Send + 'static,
    AsyncDieselConnectionManager<C>: bb8::ManageConnection,
    Pool<AsyncDieselConnectionManager<C>>: Sync + Send,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatabaseConnectionImpl").finish()
    }
}

#[async_trait]
impl<C> DatabaseConnection for DatabaseConnectionImpl<C>
where
    C: AsyncConnection + Send + 'static,
    AsyncDieselConnectionManager<C>: bb8::ManageConnection,
    Pool<AsyncDieselConnectionManager<C>>: Sync + Send,
{
    async fn get_connection(
        &self,
    ) -> Result<Box<dyn AsyncConnection + Send + 'static>> {
        let conn = self.pool.get_owned().await?;
        Ok(Box::new(conn) as Box<dyn AsyncConnection + Send + 'static>)
    }

    async fn setup_connection(&self, _conn: &mut (dyn AsyncConnection + Send + 'static)) -> Result<()> {
        // Default implementation does nothing
        Ok(())
    }
}

impl<C> DatabaseConnectionFactory for DatabaseConnectionImpl<C>
where
    C: AsyncConnection + Send + 'static,
    AsyncDieselConnectionManager<C>: bb8::ManageConnection,
    Pool<AsyncDieselConnectionManager<C>>: Sync + Send,
{
    type Connection = C;

    fn create_manager(database_url: &str) -> Result<AsyncDieselConnectionManager<C>>
    where
        Self: Sized,
    {
        Ok(AsyncDieselConnectionManager::<C>::new(database_url))
    }

    async fn setup_connection(conn: &mut Self::Connection) -> Result<()> {
        // Default implementation does nothing
        Ok(())
    }
}

/// SQLite-specific database connection implementation.
#[cfg(feature = "sqlite")]
pub type SqliteConnection = DatabaseConnectionImpl<diesel_async::AsyncSqliteConnection>;

#[cfg(feature = "sqlite")]
impl SqliteConnection {
    /// Create a new SQLite connection pool.
    pub fn new(database_url: &str) -> Result<Self> {
        use diesel_async::sqlite::AsyncSqliteConnection;
        use tokio::runtime::Runtime;

        // Create a new runtime for running async code
        let rt = Runtime::new().map_err(|e| Error::Runtime(e.to_string()))?;

        // Create the connection manager
        let manager = AsyncDieselConnectionManager::<AsyncSqliteConnection>::new(database_url);

        // Create the connection pool
        let pool = rt.block_on(async {
            bb8::Pool::builder()
                .max_size(10) // Adjust based on your needs
                .build(manager)
                .await
        })?;

        // Test the connection
        rt.block_on(async {
            let mut conn = pool.get_owned().await?;
            Self::setup_connection(&mut *conn).await?;
            Ok::<_, Error>(())
        })?;

        Ok(Self { pool: Arc::new(pool) })
    }

    /// Set up SQLite-specific connection settings
    pub async fn setup_connection(conn: &mut diesel_async::AsyncSqliteConnection) -> Result<()> {
        use diesel_async::RunQueryDsl;

        // Enable foreign key support
        diesel::sql_query("PRAGMA foreign_keys = ON")
            .execute(conn)
            .await?;

        // Enable WAL mode for better concurrency
        diesel::sql_query("PRAGMA journal_mode = WAL")
            .execute(conn)
            .await?;

        Ok(())
    }
}

/// PostgreSQL-specific database connection implementation.
#[cfg(feature = "postgres")]
pub type PostgresConnection = DatabaseConnectionImpl<diesel_async::AsyncPgConnection>;

#[cfg(feature = "postgres")]
impl PostgresConnection {
    /// Create a new PostgreSQL connection pool.
    pub fn new(database_url: &str) -> Result<Self> {
        use tokio::runtime::Runtime;

        // Create a new runtime for running async code
        let rt = Runtime::new().map_err(|e| Error::Runtime(e.to_string()))?;

        // Create the connection manager
        let manager = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(database_url);

        // Create the connection pool
        let pool = rt.block_on(async {
            bb8::Pool::builder()
                .max_size(10) // Adjust based on your needs
                .build(manager)
                .await
        })?;

        // Test the connection
        rt.block_on(async {
            let mut conn = pool.get_owned().await?;
            Self::setup_connection(&mut *conn).await?;
            Ok::<_, Error>(())
        })?;

        Ok(Self { pool: Arc::new(pool) })
    }
}

/// A wrapper around a database connection pool that provides a type-safe API.
pub struct DbConnection {
    inner: Arc<dyn DatabaseConnection + Send + Sync>,
}

impl std::fmt::Debug for DbConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbConnection").finish()
    }
}

impl Clone for DbConnection {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl DbConnection {
    /// Create a new SQLite connection.
    #[cfg(feature = "sqlite")]
    pub fn sqlite(database_url: &str) -> Result<Self> {
        let conn = SqliteConnection::new(database_url)?;
        Ok(Self {
            inner: Arc::new(conn),
        })
    }

    /// Create a new PostgreSQL connection.
    #[cfg(feature = "postgres")]
    pub fn postgres(database_url: &str) -> Result<Self> {
        let conn = PostgresConnection::new(database_url)?;
        Ok(Self {
            inner: Arc::new(conn),
        })
    }

    /// Get a connection from the pool.
    pub async fn get_connection(&self) -> Result<Box<dyn AsyncConnection + Send + 'static>> {
        self.inner.get_connection().await
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
