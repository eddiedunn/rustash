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
    
    // Check if the path itself is a directory
    if path.is_dir() {
        return Err(Error::other("Database path cannot be a directory"));
    }
    
    // On Unix-like systems, check for symlinks in the path for security
    #[cfg(unix)]
    {
        // 1. Check if the path itself is a symlink
        if path.is_symlink() {
            return Err(Error::other(format!(
                "Database path '{}' cannot be a symlink for security reasons",
                path.display()
            )));
        }

        // 2. Check if any ancestor is a symlink
        if let Some(parent) = path.parent() {
            let mut current = parent;
            loop {
                if current.is_symlink() {
                    return Err(Error::other(format!(
                        "Database path cannot be inside a symlinked directory. Ancestor '{}' is a symlink.",
                        current.display()
                    )));
                }
                if let Some(p) = current.parent() {
                    // Break if we've reached the root.
                    if p == current {
                        break;
                    }
                    current = p;
                } else {
                    break;
                }
            }
        }
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

/// Create a test database connection pool (in-memory SQLite)
pub fn create_test_pool() -> Result<DbPool> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    // Use a unique database name for each test to ensure isolation
    static TEST_DB_COUNTER: AtomicUsize = AtomicUsize::new(0);
    let test_db_number = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let database_url = format!("file:test_db_{}?mode=memory&cache=shared", test_db_number);
    
    // Create the connection pool
    let pool = DbPool::new(&database_url)?;
    
    // Set up the database
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_connection_pool() -> Result<()> {
        // Test with in-memory SQLite
        let pool = create_test_pool()?;
        assert!(pool.get().is_ok(), "Should be able to get a connection from the pool");
        
        // Test that we can get multiple connections
        let conn1 = pool.get()?;
        let conn2 = pool.get()?;
        assert_ne!(
            std::ptr::addr_of!(conn1) as *const u8,
            std::ptr::addr_of!(conn2) as *const u8,
            "Should get different connection instances"
        );
        
        Ok(())
    }
    
    #[test]
    fn test_connection_guard() -> Result<()> {
        let pool = create_test_pool()?;
        
        // Test basic guard functionality
        let mut guard = DbConnectionGuard::new(&pool)?;
        
        // Test Deref
        let _: &DbConnection = &*guard;
        
        // Test DerefMut
        let _: &mut DbConnection = &mut *guard;
        
        // Test into_inner and execute a simple query
        let mut conn = guard.into_inner();
        let result: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1"))
            .get_result(&mut conn)?;
        assert_eq!(result, 1, "Should be able to execute query on connection");
        
        Ok(())
    }
    
    #[test]
    fn test_default_db_path() -> Result<()> {
        // Test that we get a valid path
        let path = default_db_path()?;
        assert!(path.parent().is_some(), "Path should have a parent directory");
        assert_eq!(
            path.file_name().and_then(OsStr::to_str),
            Some(DEFAULT_DB_FILENAME),
            "Default filename should be used"
        );
        
        // Test that the parent directory exists or can be created
        let parent = path.parent().unwrap();
        assert!(parent.exists() || fs::create_dir_all(parent).is_ok(),
               "Should be able to create parent directory if it doesn't exist");
        
        Ok(())
    }
    
    #[test]
    fn test_validate_db_path() -> Result<()> {
        // Test with a valid path
        let temp_dir = tempdir()?;
        let valid_path = temp_dir.path().join("test.db");
        println!("Testing valid path: {:?}", valid_path);
        validate_db_path(&valid_path)?;
        
        // Test with a path that's a directory
        let dir_path = temp_dir.path();
        assert!(validate_db_path(dir_path).is_err(), "Should reject directory path");
        
        // Test with a path outside the home directory (should be allowed)
        // We can't guarantee /tmp exists on all systems, so use tempdir again.
        let outside_dir = tempdir()?;
        let outside_path = outside_dir.path().join("rustash_test.db");
        validate_db_path(&outside_path).expect("Should allow paths outside home directory");
        
        // Test with a path that is a symlink (should be rejected)
        #[cfg(unix)]
        {
            use std::os::unix::fs as unix_fs;
            
            // Create a target file for the symlink
            let target_file = temp_dir.path().join("real.db");
            fs::write(&target_file, "data")?;
            
            // Create the symlink pointing to the file
            let symlink_path = temp_dir.path().join("symlink.db");
            println!("Creating symlink from {:?} to /etc/passwd", symlink_path);
            unix_fs::symlink("/etc/passwd", &symlink_path)?;
            
            // Check if the symlink was created successfully
            let symlink_metadata = std::fs::symlink_metadata(&symlink_path);
            println!("Symlink metadata: {:?}", symlink_metadata);
            if let Ok(metadata) = symlink_metadata {
                println!("Is symlink: {}", metadata.file_type().is_symlink());
            }
            
            // Check if the path is detected as a symlink by our function
            let contains_symlink = is_path_containing_symlink(&symlink_path);
            println!("is_path_containing_symlink result: {:?}", contains_symlink);
            
            // Now test the actual validation
            let validation_result = validate_db_path(&symlink_path);
            println!("validate_db_path result: {:?}", validation_result);
            
            assert!(
                validation_result.is_err(),
                "Should reject symlink paths for security"
            );
        }
        
        Ok(())
    }
    
    #[test]
    fn test_create_connection_pool() -> Result<()> {
        // Test with default configuration (should use in-memory for tests)
        let pool = create_connection_pool()?;
        let mut conn = pool.get()?;
        
        // Test that we can execute a simple query
        let result: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1"))
            .get_result(&mut conn)?;
        assert_eq!(result, 1, "Should be able to execute query on connection");
        
        // Test with invalid database URL - use unsafe block for env var manipulation
        let result = {
            unsafe { std::env::set_var("DATABASE_URL", "file:/nonexistent/path/test.db") };
            let result = DbPool::new("file:/nonexistent/path/test.db");
            unsafe { std::env::remove_var("DATABASE_URL") };
            result
        };
        
        assert!(result.is_err(), "Should fail with invalid database path");
        
        Ok(())
    }
    
    #[test]
    fn test_connection_pool_multithreaded() -> Result<()> {
        use std::sync::Arc;
        use std::thread;
        
        let pool = Arc::new(create_test_pool()?);
        let mut handles = vec![];
        
        // Test getting connections from multiple threads
        for i in 0..5 {
            let pool_clone = Arc::clone(&pool);
            let handle = thread::spawn(move || {
                let conn = pool_clone.get();
                assert!(conn.is_ok(), "Thread {}: Failed to get connection", i);
                // Use the connection to ensure it works
                let mut conn = conn.unwrap();
                let result: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1 + 1"))
                    .get_result(&mut conn)
                    .expect("Should be able to execute query");
                assert_eq!(result, 2, "Thread {}: Query result mismatch", i);
            });
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().expect("Thread panicked");
        }
        
        Ok(())
    }
    
    #[test]
    fn test_connection_timeout() -> Result<()> {
        // Create a pool with a small number of connections and short timeout
        let url = "file::memory:?cache=shared";
        let manager = ConnectionManagerType::new(url);
        let pool = r2d2::Pool::builder()
            .max_size(1) // Only allow 1 connection
            .connection_timeout(std::time::Duration::from_millis(100))
            .build(manager)
            .map_err(|e| Error::other(format!("Failed to create connection pool: {}", e)))?;
            
        let pool = DbPool(Arc::new(pool));
        
        // Get the first connection (should succeed)
        let conn1 = pool.get();
        assert!(conn1.is_ok(), "Should be able to get first connection");
        
        // Try to get a second connection (should fail due to pool exhaustion)
        let conn2 = pool.get();
        assert!(conn2.is_err(), "Should not be able to get second connection");
        
        // Verify the error is a timeout error
        if let Err(e) = conn2 {
            assert!(
                e.to_string().contains("timeout") || 
                e.to_string().contains("timed out") ||
                e.to_string().contains("connection limit"),
                "Expected timeout or connection limit error, got: {}", e
            );
        }
        
        // Drop the first connection
        drop(conn1);
        
        // Now we should be able to get a connection again
        let conn3 = pool.get();
        assert!(conn3.is_ok(), "Should be able to get connection after previous was dropped");
        
        Ok(())
    }
}