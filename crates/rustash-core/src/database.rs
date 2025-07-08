//! Database connection management

use crate::error::{Error, Result};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager, Pool, PooledConnection};
use std::path::{Path, PathBuf};
use std::env;
use std::ffi::OsStr;
use home::home_dir;
use std::sync::Arc;
use std::ops::{Deref, DerefMut};

#[cfg(feature = "sqlite")]
use diesel::sqlite::SqliteConnection;

#[cfg(feature = "postgres")]
use diesel::pg::PgConnection;

/// Database connection type based on feature flags
#[cfg(feature = "sqlite")]
pub type DbConnection = SqliteConnection;

#[cfg(feature = "postgres")]
pub type DbConnection = PgConnection;

/// Type alias for the connection manager
#[cfg(feature = "sqlite")]
type ConnectionManagerType = ConnectionManager<SqliteConnection>;

#[cfg(feature = "postgres")]
type ConnectionManagerType = ConnectionManager<PgConnection>;

/// Type alias for the connection pool
type ConnectionPool = Pool<ConnectionManagerType>;

/// Type alias for a pooled connection
type PooledConn = PooledConnection<ConnectionManagerType>;

/// A wrapper around the connection pool that can be cloned and shared between threads
#[derive(Clone)]
pub struct DbPool(Arc<ConnectionPool>);

impl DbPool {
    /// Create a new connection pool
    pub fn new(database_url: &str) -> Result<Self> {
        let manager = ConnectionManagerType::new(database_url);
        let pool = r2d2::Pool::builder()
            .max_size(10) // Adjust based on your needs
            .build(manager)
            .map_err(|e| Error::other(format!("Failed to create connection pool: {}", e)))?;
            
        Ok(DbPool(Arc::new(pool)))
    }
    
    /// Get a connection from the pool
    pub fn get(&self) -> Result<PooledConnection<ConnectionManagerType>> {
        self.0.get().map_err(|e| Error::other(format!("Failed to get connection from pool: {}", e)))
    }
}

/// A wrapper around a pooled connection that implements `Deref` to the inner connection
pub struct DbConnectionGuard {
    conn: Option<PooledConn>,
}

impl Drop for DbConnectionGuard {
    fn drop(&mut self) {
        // The connection will be returned to the pool when dropped
    }
}

impl Deref for DbConnectionGuard {
    type Target = DbConnection;
    
    fn deref(&self) -> &Self::Target {
        self.conn.as_ref().expect("Connection already taken")
    }
}

impl DerefMut for DbConnectionGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.conn.as_mut().expect("Connection already taken")
    }
}

impl DbConnectionGuard {
    /// Create a new connection guard
    pub fn new(pool: &DbPool) -> Result<Self> {
        let conn = pool.get()?;
        Ok(Self {
            conn: Some(conn),
        })
    }
    
    /// Explicitly get the inner connection
    pub fn into_inner(mut self) -> PooledConn {
        self.conn.take().expect("Connection already taken")
    }
    
    /// Test if the database connection is still valid
    pub fn test_connection(&mut self) -> Result<()> {
        use diesel::connection::SimpleConnection;
        
        // Execute a simple query to test the connection
        self.conn
            .as_mut()
            .expect("Connection already taken")
            .batch_execute("SELECT 1")
            .map_err(|e| Error::other(format!("Connection test failed: {}", e)))?;
            
        Ok(())
    }
}

/// Default database filename
const DEFAULT_DB_FILENAME: &str = "rustash.db";

/// Get the default database path in the user's config directory
fn default_db_path() -> Result<PathBuf> {
    let home = home_dir().ok_or_else(|| Error::other("Could not determine home directory"))?;
    let mut path = home.join(".config").join("rustash");
    
    // Create the directory if it doesn't exist
    if !path.exists() {
        std::fs::create_dir_all(&path).map_err(|e| {
            Error::other(format!("Failed to create config directory: {}", e))
        })?;
    }
    
    path.push(DEFAULT_DB_FILENAME);
    Ok(path)
}

/// Validate that the database path is safe to use
fn validate_db_path(path: &Path) -> Result<()> {
    // Check if the path is absolute
    if !path.is_absolute() {
        return Err(Error::other("Database path must be absolute"));
    }
    
    // Prevent using special files or devices
    if path.file_name().and_then(OsStr::to_str).map_or(true, |name| name.is_empty()) {
        return Err(Error::other("Invalid database filename"));
    }
    
    // Get the parent directory
    let parent = path.parent().ok_or_else(|| Error::other("Invalid database path"))?;
    
    // Only perform write check if directory doesn't exist and needs to be created
    if !parent.exists() {
        // Directory doesn't exist, create it and we know it's writable
        std::fs::create_dir_all(parent).map_err(|e| {
            Error::other(format!("Failed to create database directory: {}", e))
        })?;
    } else {
        // Directory exists, just check metadata (faster than write test)
        let metadata = std::fs::metadata(parent).map_err(|e| {
            Error::other(format!("Cannot access database directory: {}", e))
        })?;
        
        if !metadata.is_dir() {
            return Err(Error::other("Database path must be a directory"));
        }
    }
    
    // Skip the write test for performance - we'll get a clear error on database creation
    // if there are permission issues
    
    Ok(())
}

/// Create a new database connection pool
pub fn create_connection_pool() -> Result<DbPool> {
    // Get database URL from environment or use default
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        // If no DATABASE_URL is set, use the default path
        default_db_path()
            .expect("Failed to get default database path")
            .to_str()
            .expect("Database path is not valid UTF-8")
            .to_string()
    });
    
    // For SQLite, ensure the path is absolute and the parent directory exists
    if database_url.starts_with("file:") || !database_url.contains(":") {
        let path = Path::new(&database_url);
        validate_db_path(path)?;
        
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    Error::other(format!(
                        "Failed to create database directory {}: {}",
                        parent.display(),
                        e
                    ))
                })?;
            }
        }
    }
    
    // Create the connection pool
    let pool = DbPool::new(&database_url)?;
    
    // Test the connection and enable foreign keys for SQLite
    {
        let mut conn = pool.get()?;
        
        // Enable foreign key support for SQLite
        #[cfg(feature = "sqlite")]
        diesel::sql_query("PRAGMA foreign_keys = ON")
            .execute(&mut *conn)
            .map_err(|e| Error::other(format!("Failed to enable foreign keys: {}", e)))?;
    }
    
    Ok(pool)
}

/// Establish a single database connection (for backward compatibility)
/// Note: This creates a new connection pool each time, which is not efficient.
/// Prefer using DbPool and getting connections from it instead.
pub fn establish_connection() -> Result<DbConnection> {
    let _pool = create_connection_pool()?;
    // The connection is not used directly, we create a new one below
    let _conn = _pool.get()?;
    // Return a new connection instead of trying to extract from the pooled connection
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| default_db_path().unwrap().to_string_lossy().into_owned());
    #[cfg(feature = "sqlite")]
    return Ok(SqliteConnection::establish(&database_url)?);
    
    #[cfg(feature = "postgres")]
    return Ok(PgConnection::establish(&database_url)?);
}

/// Create a test database connection pool (in-memory SQLite)
pub fn create_test_pool() -> Result<DbPool> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    // Use a unique database name for each test
    static TEST_DB_COUNTER: AtomicUsize = AtomicUsize::new(0);
    let test_db_number = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let database_url = format!("file:test_db_{}?mode=memory&cache=shared", test_db_number);
    
    // Create the connection pool
    let pool = DbPool::new(&database_url)?;
    
    // Test the connection and set up the database
    {
        let mut conn = pool.get()?;
        
        // Enable foreign keys and WAL mode for better performance
        diesel::sql_query("PRAGMA foreign_keys = ON")
            .execute(&mut *conn)
            .map_err(|e| Error::other(format!("Failed to enable foreign keys: {}", e)))?;
            
        diesel::sql_query("PRAGMA journal_mode = WAL")
            .execute(&mut *conn)
            .map_err(|e| Error::other(format!("Failed to enable WAL mode: {}", e)))?;
        
        // Run migrations using Diesel's embedded migrations
        #[cfg(feature = "sqlite")]
        {
            use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
            use std::path::Path;
            
            // This will include the migrations at compile time
            // The path is relative to the crate root (where Cargo.toml is located)
            pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
            
            // Run the migrations
            conn.run_pending_migrations(MIGRATIONS).map_err(|e| {
                Error::other(format!("Failed to run migrations on test database: {}", e))
            })?;
        }
    }
    
    Ok(pool)
}

/// Establish a test database connection (for backward compatibility)
/// Note: This creates a new test pool each time, which is not efficient.
/// Prefer using create_test_pool and getting connections from it instead.
pub fn establish_test_connection() -> Result<DbConnection> {
    // For test connections, we'll just create a new in-memory SQLite connection
    #[cfg(feature = "sqlite")]
    return Ok(SqliteConnection::establish("file::memory:?cache=shared")?);
    
    #[cfg(feature = "postgres")]
    return Ok(PgConnection::establish("postgres://localhost/test")?);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_pool() -> Result<()> {
        let pool = create_test_pool()?;
        
        // Test getting a connection from the pool and executing a simple query
        let mut conn = pool.get()?;
        let result: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1"))
            .get_result(&mut *conn)?;
        assert_eq!(result, 1);
        
        // Test multiple connections
        let mut conn2 = pool.get()?;
        let result: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1"))
            .get_result(&mut *conn2)?;
        assert_eq!(result, 1);
        
        Ok(())
    }

    #[test]
    fn test_connection_guard() -> Result<()> {
        let pool = create_test_pool()?;
        let mut guard = DbConnectionGuard::new(&pool)?;
        
        // Test deref and deref_mut by executing a simple query
        let result: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1"))
            .get_result(&mut *guard)?;
        assert_eq!(result, 1);
        
        // Test into_inner
        let mut conn = guard.into_inner();
        let result: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1"))
            .get_result(&mut *conn)?;
        assert_eq!(result, 1);
        
        Ok(())
    }
}