//! Database connection management for the CLI

use crate::main::DatabaseBackend;
use anyhow::{Context, Result};
use rustash_core::database::{create_connection_pool, DbPool, DbConnectionGuard};
use std::path::PathBuf;
use std::sync::Arc;
use std::str::FromStr;

lazy_static::lazy_static! {
    static ref DB_POOL: std::sync::Mutex<Option<Arc<DbPool>>> = std::sync::Mutex::new(None);
}

/// Initialize the database connection pool with the specified backend
pub fn init(backend: DatabaseBackend, db_path: Option<PathBuf>) -> Result<()> {
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
    std::env::set_var("DATABASE_URL", &database_url);
    
    let pool = Arc::new(create_connection_pool()?);
    *DB_POOL.lock().unwrap() = Some(pool);
    
    Ok(())
}

/// Get a database connection from the pool
pub fn get_connection() -> anyhow::Result<DbConnectionGuard> {
    // Store the guard in a variable to extend its lifetime
    let pool_guard = DB_POOL.lock().unwrap();
    let pool = pool_guard
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Database pool not initialized"))?;
    
    let guard = DbConnectionGuard::new(pool)?;
    Ok(guard)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustash_core::create_test_pool;

    #[test]
    fn test_connection_pool() -> anyhow::Result<()> {
        // Initialize with test pool
        let test_pool = Arc::new(create_test_pool()?);
        *DB_POOL.lock().unwrap() = Some(test_pool);

        // Get a connection
        let conn = get_connection()?;
        assert!(conn.test_connection().is_ok());
        
        Ok(())
    }
}
