//! Database connection and pooling functionality for Rustash.
//!
//! This module provides database connection pooling and management for different
//! database backends (SQLite and PostgreSQL) with compile-time backend selection.

use crate::error::{Error, Result};
use diesel::migration::MigrationConnection;
use diesel_migrations::embed_migrations;

// A common MIGRATIONS constant that can be used by backend-specific modules.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

// Re-export the migration types for use in backend modules
pub use diesel_migrations::{EmbeddedMigrations, MigrationHarness};



#[cfg(feature = "sqlite")]
pub mod sqlite_pool {
    use super::*;
    use diesel_async::sqlite::AsyncSqliteConnection;

    pub type SqlitePool = bb8::Pool<
        diesel_async::pooled_connection::AsyncDieselConnectionManager<AsyncSqliteConnection>,
    >;

    pub async fn create_pool(database_url: &str) -> Result<SqlitePool> {
        let manager = diesel_async::pooled_connection::AsyncDieselConnectionManager::<
            AsyncSqliteConnection,
        >::new(database_url);
        let pool = bb8::Pool::builder()
            .build(manager)
            .await
            .map_err(|e| Error::Pool(e.to_string()))?;

        // Run migrations on a new connection from the pool
        let mut conn = pool.get().await.map_err(|e| Error::Pool(e.to_string()))?;
        conn.interact(|conn| conn.run_pending_migrations(MIGRATIONS).map(|_| ()))
            .await
            .map_err(|e| Error::Other(format!("Migration task failed: {}", e)))??;

        Ok(pool)
    }
}

#[cfg(feature = "postgres")]
pub mod postgres_pool {
    use super::*;
    use diesel_async::AsyncPgConnection;

    pub type PgPool =
        bb8::Pool<diesel_async::pooled_connection::AsyncDieselConnectionManager<AsyncPgConnection>>;

    pub async fn create_pool(database_url: &str) -> Result<PgPool> {
        let manager = diesel_async::pooled_connection::AsyncDieselConnectionManager::<
            AsyncPgConnection,
        >::new(database_url);
        let pool = bb8::Pool::builder()
            .build(manager)
            .await
            .map_err(|e| Error::Pool(e.to_string()))?;

        // Run migrations on a new connection from the pool
        let mut conn = pool.get().await.map_err(|e| Error::Pool(e.to_string()))?;
        conn.interact(|conn| conn.run_pending_migrations(MIGRATIONS).map(|_| ()))
            .await
            .map_err(|e| Error::Other(format!("Migration task failed: {}", e)))??;

        Ok(pool)
    }
}

/// Create a test database connection pool for integration tests.
#[cfg(feature = "sqlite")]
pub async fn create_test_pool() -> Result<sqlite_pool::SqlitePool> {
    // For tests, always use an in-memory SQLite database.
    let pool = sqlite_pool::create_pool(":memory:").await?;
    Ok(pool)
}

#[cfg(all(test, not(feature = "sqlite")))]
pub async fn create_test_pool() -> Result<()> {
    panic!("The 'sqlite' feature must be enabled to run tests that use create_test_pool.");
}
