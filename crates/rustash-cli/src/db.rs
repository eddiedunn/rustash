//! Database connection management for the CLI

use crate::DatabaseBackend;
use anyhow::{Context, Result};
use rustash_core::database::{create_connection_pool, DbPool, DbConnectionGuard};
use std::path::PathBuf;
use std::sync::Arc;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};

// This will include the migrations at compile time
// The migrations are in the rustash-core crate
const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../rustash-core/migrations");

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
    // This is unsafe because it's not thread-safe, but it's acceptable here because:
    // 1. It's called once at application startup
    // 2. It's before any threads are spawned
    // 3. The value is constant for the lifetime of the application
    unsafe {
        std::env::set_var("DATABASE_URL", &database_url);
    }
    
    let pool = Arc::new(create_connection_pool()?);
    
    // Run migrations
    {
        let conn = &mut pool.get()?;
        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|e| anyhow::anyhow!("Failed to run migrations: {}", e))?;
        log::info!("Successfully ran database migrations");
    }
    
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
    use rustash_core::database::create_test_pool;
    use diesel::prelude::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
    use std::error::Error;
    
    // Struct to hold the result of the COUNT query
    #[derive(QueryableByName)]
    struct TableCount {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        count: i32,
    }
    
    // This will include the migrations at compile time
    // The migrations are in the rustash-core crate
    // Use the path relative to the rustash-core crate
    pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../rustash-core/migrations");

    // Helper function to run migrations on a connection
    fn run_migrations(conn: &mut impl MigrationHarness<diesel::sqlite::Sqlite>) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        conn.run_pending_migrations(MIGRATIONS)?;
        Ok(())
    }

    #[test]
    fn test_connection_pool() -> anyhow::Result<()> {
        // Create a test pool
        let test_pool = Arc::new(create_test_pool()?);
        
        // Store the pool in the global state
        *DB_POOL.lock().unwrap() = Some(test_pool.clone());

        // Get a connection
        let mut conn = get_connection()?;
        
        // Ensure the connection is valid
        assert!(conn.test_connection().is_ok());
        
        // Check if the snippets table exists
        let table_exists = diesel::sql_query(
            "SELECT COUNT(*) as count FROM sqlite_master WHERE type='table' AND name='snippets'"
        )
        .get_result::<TableCount>(&mut *conn)
        .map(|r| r.count)
        .unwrap_or(0);
        
        if table_exists == 0 {
            // If the table doesn't exist, try to run migrations
            eprintln!("Snippets table not found. Running migrations...");
            run_migrations(&mut *conn).map_err(|e| {
                eprintln!("Failed to run migrations: {}", e);
                anyhow::anyhow!("Failed to run migrations: {}", e)
            })?;
            
            // Verify the table was created
            let table_exists_after = diesel::sql_query(
                "SELECT COUNT(*) as count FROM sqlite_master WHERE type='table' AND name='snippets'"
            )
            .get_result::<TableCount>(&mut *conn)
            .map(|r| r.count)
            .unwrap_or(0);
            
            assert_ne!(table_exists_after, 0, "Snippets table should exist after running migrations");
        }
        
        Ok(())
    }
}
