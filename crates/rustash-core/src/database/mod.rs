//! Database connection and pooling functionality for Rustash.
//!
//! This module provides database connection pooling and management for different
//! database backends (SQLite and PostgreSQL) with compile-time backend selection.

use crate::error::{Error, Result};
use diesel_migrations::embed_migrations;

// A common MIGRATIONS constant that can be used by backend-specific modules.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

// Re-export the migration types for use in backend modules
pub use diesel_migrations::EmbeddedMigrations;

#[cfg(feature = "sqlite")]
pub mod sqlite_pool {
    use super::*;
    use diesel::SqliteConnection;
    use diesel_async::pooled_connection::bb8::Pool;
    use diesel_async::pooled_connection::AsyncDieselConnectionManager;
    use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;
    use diesel_migrations::MigrationHarness;

    pub type SqlitePool = Pool<SyncConnectionWrapper<SqliteConnection>>;

    pub async fn create_pool(database_url: &str) -> Result<SqlitePool> {
        let manager = AsyncDieselConnectionManager::<SyncConnectionWrapper<SqliteConnection>>::new(
            database_url,
        );
        let pool = Pool::builder()
            .build(manager)
            .await
            .map_err(|e| Error::Pool(e.to_string()))?;

        // Run migrations on a new connection from the pool
        let mut conn = pool.get().await.map_err(|e| Error::Pool(e.to_string()))?;
        (&mut *conn)
            .run_pending_migrations(MIGRATIONS)
            .map_err(|e| Error::Other(format!("Migration failed: {}", e)))?;

        Ok(pool)
    }
}

#[cfg(feature = "postgres")]
pub mod postgres_pool {
    use super::*;
    use diesel_async::pg::AsyncPgConnection;
    use diesel_async::pooled_connection::bb8::Pool;
    use diesel_async::pooled_connection::AsyncDieselConnectionManager;
    use diesel_migrations::MigrationHarness;

    pub type PgPool = Pool<AsyncPgConnection>;

    pub async fn create_pool(database_url: &str) -> Result<PgPool> {
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
        let pool = Pool::builder()
            .build(manager)
            .await
            .map_err(|e| Error::Pool(e.to_string()))?;

        // Run migrations on a new connection from the pool
        let mut conn = pool.get().await.map_err(|e| Error::Pool(e.to_string()))?;
        (&mut *conn)
            .run_pending_migrations(MIGRATIONS)
            .map_err(|e| Error::Other(format!("Migration failed: {}", e)))?;

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
