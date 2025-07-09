//! Database connection management for the CLI

use crate::DatabaseBackend;
use anyhow::{Context, Result};
use rustash_core::database::{self, DbConnection, DbPool};
use std::path::PathBuf;
use std::sync::Arc;

static DB_POOL: tokio::sync::OnceCell<Arc<DbPool>> = tokio::sync::OnceCell::const_new();

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

    let pool = Arc::new(DbPool::new(&database_url).await?);

    database::run_migrations(&pool).await?;

    DB_POOL
        .set(pool)
        .map_err(|_| anyhow::anyhow!("Database pool already initialized"))?;

    Ok(())
}

/// Get a database connection from the pool
pub async fn get_connection() -> anyhow::Result<DbConnection> {
    let pool = get_pool().await?;
    let conn = pool.get_async().await?;
    Ok(conn)
}

/// Get a clone of the global connection pool
pub async fn get_pool() -> anyhow::Result<Arc<DbPool>> {
    DB_POOL
        .get()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Database pool not initialized"))
}
