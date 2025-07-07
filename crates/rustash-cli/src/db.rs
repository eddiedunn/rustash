//! Database connection management for the CLI

use rustash_core::database::{create_connection_pool, DbPool, DbConnectionGuard};
use std::sync::Arc;

lazy_static::lazy_static! {
    static ref DB_POOL: std::sync::Mutex<Option<Arc<DbPool>>> = std::sync::Mutex::new(None);
}

/// Initialize the database connection pool
pub fn init() -> anyhow::Result<()> {
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
