//! Database connection management for the CLI

use crate::DatabaseBackend;
use anyhow::{Context, Result};
use diesel::prelude::*;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use rustash_core::database::{DbConnection, DbPool, create_connection_pool};
use std::path::PathBuf;
use std::sync::Arc;

// This will include the migrations at compile time
// The migrations are in the rustash-core crate
const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../rustash-core/migrations");

lazy_static::lazy_static! {
    static ref DB_POOL: std::sync::Mutex<Option<Arc<DbPool>>> = std::sync::Mutex::new(None);
}

/// Initialize the database connection pool with the specified backend
pub async fn init(backend: DatabaseBackend, db_path: Option<PathBuf>) -> Result<()> {
    let database_url = match backend {
        DatabaseBackend::Sqlite => {
            let path = db_path.unwrap_or_else(|| {
                dirs::data_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("rustash")
                    .join("rustash.db")
            });
            // Ensure the directory exists
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create database directory: {}", parent.display())
                })?;
            }
            format!("sqlite://{}", path.display())
        }
        DatabaseBackend::Postgres => {
            db_path
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .or_else(|| std::env::var("DATABASE_URL").ok())
                .ok_or_else(|| anyhow::anyhow!("PostgreSQL connection string must be provided either via --db-path or DATABASE_URL environment variable"))?
        }
    };

    // Set the DATABASE_URL environment variable for diesel_cli compatibility
    // This is unsafe because it's not thread-safe, but it's acceptable here because:
    // 1. It's called once at application startup
    // 2. It's before any threads are spawned
    // 3. The value is constant for the lifetime of the application
    unsafe {
        std::env::set_var("DATABASE_URL", &database_url);
    }

    let pool = Arc::new(create_connection_pool().await?);

    // Run migrations
    {
        let mut conn = DbConnection::from(pool.get().await?);
        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|e| anyhow::anyhow!("Failed to run migrations: {}", e))?;
        log::info!("Successfully ran database migrations");
    }

    *DB_POOL.lock().unwrap() = Some(pool);

    Ok(())
}

/// Get a database connection from the pool
pub async fn get_connection() -> anyhow::Result<DbConnection> {
    let pool_guard = DB_POOL.lock().unwrap();
    let pool = pool_guard
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Database pool not initialized"))?;

    let conn = pool
        .get()
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    Ok(DbConnection::from(conn))
}
