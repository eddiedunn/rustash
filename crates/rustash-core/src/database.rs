//! Database connection management

use crate::error::{Error, Result};
use std::path::{Path, PathBuf};
use std::ffi::OsStr;
use home::home_dir;
use std::sync::Arc;
use std::ops::{Deref, DerefMut};
use diesel::RunQueryDsl;

// Print which features are enabled for debugging
#[cfg(feature = "sqlite")]
log::debug!("SQLite feature is enabled");

#[cfg(feature = "postgres")]
log::debug!("PostgreSQL feature is enabled");

// Ensure exactly one database backend is enabled
#[cfg(all(feature = "sqlite", feature = "postgres"))]
compile_error!("Features 'sqlite' and 'postgres' are mutually exclusive - enable only one");

#[cfg(not(any(feature = "sqlite", feature = "postgres")))]
compile_error!("Either feature 'sqlite' or 'postgres' must be enabled");

// Database connection types
mod connection {
    use super::*;
    
    // Use cfg_if to handle the mutually exclusive features
    cfg_if::cfg_if! {
        if #[cfg(feature = "sqlite")] {
            use diesel::sqlite::SqliteConnection;
            use diesel::r2d2::{self, ConnectionManager};
            
            pub type Connection = SqliteConnection;
            pub type Manager = ConnectionManager<Connection>;
            pub type Pool = r2d2::Pool<Manager>;
            pub type PooledConn = r2d2::PooledConnection<Manager>;
        } else if #[cfg(feature = "postgres")] {
            use diesel::pg::PgConnection;
            use diesel_async::{
                pooled_connection::AsyncDieselConnectionManager,
                AsyncPgConnection,
                async_connection_wrapper::AsyncConnectionWrapper,
            };
            use bb8::Pool as Bb8Pool;
            use std::convert::Infallible;
            use async_trait::async_trait;
            
            pub type Connection = PgConnection;
            pub type AsyncConnection = AsyncPgConnection;
            pub type Manager = AsyncDieselConnectionManager<AsyncPgConnection>;
            pub type Pool = Bb8Pool<Manager>;
            pub type PooledConn = AsyncConnectionWrapper<AsyncPgConnection>;
            
            // Wrapper to provide a sync interface over async connection
            pub struct SyncConnectionWrapper(PooledConn);
            
            #[async_trait]
            impl diesel_async::AsyncConnection<diesel_async::AsyncPgConnection> for SyncConnectionWrapper {
                type TransactionManager = <AsyncPgConnection as diesel_async::AsyncConnection<diesel_async::AsyncPgConnection>>::TransactionManager;
                
                fn get_transaction_manager(&self) -> &Self::TransactionManager {
                    self.0.get_transaction_manager()
                }
                
                async fn transaction<T, E, F>(&mut self, callback: F) -> Result<T, E>
                where
                    F: std::future::Future<Output = Result<T, E>> + Send,
                    E: From<diesel::result::Error> + From<Infallible>,
                    T: Send,
                {
                    self.0.transaction(callback).await
                }
            }
        } else {
            // This will be caught by the compile_error! at the top of the file
            pub type Connection = ();
            pub type Manager = ();
            pub type Pool = ();
            pub type PooledConn = ();
        }
    }
}

// Re-export the connection types
pub use connection::Connection as DbConnection;
pub use connection::Pool as DbPool;
pub use connection::PooledConn;

type ConnectionManagerType = connection::Manager;

// Import the appropriate pool type based on the backend
#[cfg(feature = "sqlite")]
use diesel::r2d2::{Pool, PooledConnection};

#[cfg(feature = "postgres")]
use bb8::Pool as Bb8Pool;

/// A wrapper around the connection pool that can be cloned and shared between threads
#[derive(Clone)]
pub struct DbPool(Arc<dyn std::any::Any + Send + Sync>);

// Manually implement Debug since we can't derive it for trait objects
impl std::fmt::Debug for DbPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbPool").finish()
    }
}

impl DbPool {
    /// Create a new connection pool
    pub async fn new(database_url: &str) -> Result<Self> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "sqlite")] {
                use diesel::r2d2::ConnectionManager;
                use diesel::SqliteConnection;
                
                let manager = ConnectionManager::<SqliteConnection>::new(database_url);
                let pool = Pool::builder()
                    .build(manager)
                    .map_err(|e| Error::other(format!("Failed to create SQLite connection pool: {}", e)))?;
                
                Ok(Self(Arc::new(pool)))
            } else if #[cfg(feature = "postgres")] {
                use diesel_async::AsyncDieselConnectionManager;
                use diesel_async::AsyncPgConnection;
                
                let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
                let pool = Bb8Pool::builder()
                    .build(manager)
                    .await
                    .map_err(|e| Error::other(format!("Failed to create PostgreSQL connection pool: {}", e)))?;
                
                Ok(Self(Arc::new(pool)))
            } else {
                compile_error!("Either 'sqlite' or 'postgres' feature must be enabled");
            }
        }
    }

    /// Get a connection from the pool
    pub async fn get_async(&self) -> Result<DbConnection> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "sqlite")] {
                use diesel::r2d2::PooledConnection;
                use diesel::SqliteConnection;
                
                // For SQLite, we need to use a blocking task
                let pool = self.0.downcast_ref::<Pool<ConnectionManager<SqliteConnection>>>()
                    .ok_or_else(|| Error::other("Failed to downcast SQLite connection pool"))?;
                
                let pool = pool.clone();
                let conn = tokio::task::spawn_blocking(move || {
                    pool.get()
                })
                .await
                .map_err(|e| Error::other(format!("Failed to get SQLite connection: {}", e)))??;
                
                Ok(Box::new(conn))
            } else if #[cfg(feature = "postgres")] {
                use bb8::PooledConnection;
                use diesel_async::AsyncPgConnection;
                
                // For PostgreSQL, we can use the async pool directly
                let pool = self.0.downcast_ref::<Bb8Pool<AsyncDieselConnectionManager<AsyncPgConnection>>>()
                    .ok_or_else(|| Error::other("Failed to downcast PostgreSQL connection pool"))?;
                
                let conn = pool.get().await
                    .map_err(|e| Error::other(format!("Failed to get PostgreSQL connection: {}", e)))?;
                
                Ok(Box::new(conn))
            } else {
                compile_error!("Either 'sqlite' or 'postgres' feature must be enabled");
            }
        }
    }
    
    /// Get a connection from the pool (synchronous interface)
    pub fn get(&self) -> Result<DbConnectionGuard> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "sqlite")] {
                use diesel::r2d2::PooledConnection;
                use diesel::SqliteConnection;
                
                // For SQLite, we can get a connection directly
                let pool = self.0.downcast_ref::<Pool<ConnectionManager<SqliteConnection>>>()
                    .ok_or_else(|| Error::other("Failed to downcast SQLite connection pool"))?;
                
                let conn = pool.get()
                    .map_err(|e| Error::other(format!("Failed to get SQLite connection: {}", e)))?;
                
                Ok(DbConnectionGuard {
                    #[cfg(feature = "sqlite")]
                    conn: Some(Box::new(conn)),
                    #[cfg(feature = "postgres")]
                    conn: None,
                    pool: self.clone(),
                })
            } else if #[cfg(feature = "postgres")] {
                // For PostgreSQL, we need to block on the async operation
                let rt = tokio::runtime::Runtime::new()
                    .map_err(|e| Error::other(format!("Failed to create runtime: {}", e)))?;
                
                let pool = self.clone();
                let conn = rt.block_on(async {
                    pool.get_async().await
                })?;
                
                Ok(DbConnectionGuard {
                    #[cfg(feature = "sqlite")]
                    conn: None,
                    #[cfg(feature = "postgres")]
                    conn: Some(conn),
                    pool: self.clone(),
                })
            } else {
                compile_error!("Either 'sqlite' or 'postgres' feature must be enabled");
            }
        }
    }
}

/// A wrapper around a pooled connection that implements `Deref` to the inner connection
pub struct DbConnectionGuard {
    #[cfg(feature = "sqlite")]
    conn: Option<Box<dyn std::ops::Deref<Target = diesel::SqliteConnection> + Send + 'static>>,
    #[cfg(feature = "postgres")]
    conn: Option<Box<dyn std::ops::Deref<Target = diesel_async::AsyncPgConnection> + Send + 'static>>,
    _pool: DbPool, // Keep the pool alive while the guard is alive
}

impl std::fmt::Debug for DbConnectionGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbConnectionGuard").finish()
    }
}

impl Drop for DbConnectionGuard {
    fn drop(&mut self) {
        // The connection will be returned to the pool when dropped
    }
}

impl std::ops::Deref for DbConnectionGuard {
    type Target = DbConnection;
    
    fn deref(&self) -> &Self::Target {
        // SAFETY: We know the connection exists because we create it in new()
        self.conn.as_ref().map(|c| c.deref() as &DbConnection).unwrap()
    }
}

impl std::ops::DerefMut for DbConnectionGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: We know the connection exists because we create it in new()
        self.conn.as_mut().map(|c| c.deref_mut() as &mut DbConnection).unwrap()
    }
}

impl DbConnectionGuard {
    /// Create a new connection guard
    pub(crate) fn new(conn: DbConnection, pool: DbPool) -> Self {
        Self {
            conn: Some(conn),
            _pool: pool,
        }
    }

    /// Explicitly get the inner connection
    pub fn into_inner(mut self) -> DbConnection {
        self.conn.take().unwrap()
    }
    
    /// Test if the database connection is still valid
    pub fn test_connection(&mut self) -> Result<()> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "sqlite")] {
                use diesel::sql_query;
                use diesel::RunQueryDsl;
                
                // For SQLite, we can execute the query directly
                sql_query("SELECT 1").execute(&mut **self)?;
                Ok(())
            } else if #[cfg(feature = "postgres")] {
                use diesel::sql_query;
                use diesel_async::RunQueryDsl;
                
                // For PostgreSQL, we need to use a runtime to execute the query
                let rt = tokio::runtime::Runtime::new()
                    .map_err(|e| Error::other(format!("Failed to create runtime: {}", e)))?;
                
                rt.block_on(async {
                    let conn = self.conn.as_mut().unwrap();
                    sql_query("SELECT 1").execute(&mut **conn).await?;
                    Ok::<_, Error>(())
                })
            } else {
                compile_error!("Either 'sqlite' or 'postgres' feature must be enabled");
            }
        }
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
        use std::env;
        
        // 1. Check if the path itself is a symlink
        if path.is_symlink() {
            return Err(Error::other(format!(
                "Database path '{}' cannot be a symlink for security reasons",
                path.display()
            )));
        }

        // 2. Check if any ancestor is a symlink, but allow symlinks in the system temp dir
        if let Some(parent) = path.parent() {
            let mut current = parent;
            let temp_dir = std::env::temp_dir();
            
            loop {
                // Skip symlink check if we're in the system temp directory
                if temp_dir.starts_with(current) || current.starts_with(&temp_dir) {
                    break;
                }
                
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

/// 
/// For SQLite, this creates an in-memory database.
/// For PostgreSQL, this creates a connection to a local test database.
/// The database name includes a unique number to ensure test isolation.
pub async fn create_test_pool() -> Result<DbPool> {
    // Use a unique database name for each test run to avoid conflicts
    let test_db_number = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    
    // Create the database URL based on the backend
    let (database_url, test_db_name) = if cfg!(feature = "sqlite") {
        // For SQLite, use an in-memory database with a unique name
        let db_url = format!("file:test_db_{}?mode=memory&cache=shared", test_db_number);
        (db_url, String::new())
    } else if cfg!(feature = "postgres") {
        // For PostgreSQL, we'll create a new test database
        let test_db_name = format!("test_db_{}", test_db_number);
        let db_url = format!("postgres://postgres:postgres@localhost/{}", test_db_name);
        
        // First, connect to the default postgres database to create our test database
        let admin_url = "postgres://postgres:postgres@localhost/postgres";
        let admin_pool = DbPool::new(admin_url).await?;
        
        // Create the test database
        {
            let mut conn = admin_pool.get_async().await?;
            
            // Terminate any existing connections to the test database
            diesel::sql_query(format!(
                "SELECT pg_terminate_backend(pg_stat_activity.pid) \
                FROM pg_stat_activity \
                WHERE pg_stat_activity.datname = '{}' \
                AND pid <> pg_backend_pid()", test_db_name
            )).execute(&mut *conn).await?;
            
            // Drop the database if it exists
            diesel::sql_query(format!("DROP DATABASE IF EXISTS {}", test_db_name))
                .execute(&mut *conn)
                .await?;
                
            // Create a new test database
            diesel::sql_query(format!("CREATE DATABASE {}", test_db_name))
                .execute(&mut *conn)
                .await?;
        }
        
        (db_url, test_db_name)
    } else {
        return Err(Error::other("No database backend feature enabled"));
    };
    
    // Create the connection pool to the test database
    let pool = DbPool::new(&database_url).await?;
    
    // Set up the database
    {
        let mut conn = pool.get_async().await?;
        
        // Database-specific setup
        if cfg!(feature = "sqlite") {
            // SQLite-specific setup
            diesel::sql_query("PRAGMA foreign_keys = ON")
                .execute(&mut *conn)
                .await
                .map_err(|e| Error::other(format!("Failed to enable foreign keys: {}", e)))?;
                
            diesel::sql_query("PRAGMA journal_mode = WAL")
                .execute(&mut *conn)
                .await
                .map_err(|e| Error::other(format!("Failed to enable WAL mode: {}", e)))?;
        } else if cfg!(feature = "postgres") {
            // PostgreSQL-specific setup
            // Enable required extensions
            diesel::sql_query("CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\"")
                .execute(&mut *conn)
                .await
                .map_err(|e| Error::other(format!("Failed to create uuid-ossp extension: {}", e)))?;
                
            diesel::sql_query("CREATE EXTENSION IF NOT EXISTS \"pgcrypto\"")
                .execute(&mut *conn)
                .await
                .map_err(|e| Error::other(format!("Failed to create pgcrypto extension: {}", e)))?;
        }
        
        // Run migrations using Diesel's embedded migrations
        use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
        
        // This will include the migrations at compile time
        // The path is relative to the crate root (where Cargo.toml is located)
        const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
        
        // Run the migrations
        conn.run_pending_migrations(MIGRATIONS)
            .await
            .map_err(|e| Error::other(format!("Failed to run migrations: {}", e)))?;
    }
    
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use tokio::runtime::Runtime;
    
    // Helper function to check if we're running with PostgreSQL
    fn is_postgres() -> bool {
        cfg!(feature = "postgres")
    }
    
    // Helper function to run async tests
    fn run_async_test<F, Fut>(f: F) -> Result<()>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        let rt = Runtime::new().map_err(|e| Error::other(format!("Failed to create runtime: {}", e)))?;
        rt.block_on(f())
    }

    #[test]
    fn test_connection_pool() -> Result<()> {
        run_async_test(|| async {
            let pool = create_test_pool().await?;
            assert!(pool.get().is_ok(), "Should be able to get a connection from the pool");
            
            // Test that we can get multiple connections
            let conn1 = pool.get()?;
            let conn2 = pool.get()?;
            
            // For PostgreSQL, we can't compare connection addresses directly as they might be pooled
            if !is_postgres() {
                assert_ne!(
                    std::ptr::addr_of!(conn1) as *const u8,
                    std::ptr::addr_of!(conn2) as *const u8,
                    "Should get different connection instances"
                );
            }
            
            Ok(())
        })
    }
    
    #[test]
    fn test_connection_guard() -> Result<()> {
        run_async_test(|| async {
            let pool = create_test_pool().await?;
            
            // Test that we can get a connection guard
            let guard = pool.get()?;
            assert!(guard.test_connection().is_ok(), "Connection should be valid");
            
            // Test that the guard can be converted back to a connection
            let _conn = guard.into_inner();
            
            Ok(())
        })
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
            
            // Test the actual validation
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
        run_async_test(|| async {
            // Test with default configuration
            let pool = create_connection_pool()?;
            let mut conn = pool.get()?;
            
            // Test that we can execute a simple query
            let result: i32 = if is_postgres() {
                diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1 + 0"))
                    .get_result(&mut conn)?
            } else {
                diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1"))
                    .get_result(&mut conn)?
            };
            assert_eq!(result, 1, "Should be able to execute query on connection");
            
            // Test with invalid database URL
            let invalid_url = if is_postgres() {
                "postgres://invalid:invalid@localhost/nonexistent"
            } else {
                "file:/nonexistent/path/test.db"
            };
            
            let result = {
                unsafe { std::env::set_var("DATABASE_URL", invalid_url) };
                let result = DbPool::new(invalid_url);
                unsafe { std::env::remove_var("DATABASE_URL") };
                result
            };
            
            assert!(result.is_err(), "Should fail with invalid database URL");
            
            Ok(())
        })
    }
    
    #[test]
    fn test_connection_pool_multithreaded() -> Result<()> {
        run_async_test(|| async {
            use std::sync::Arc;
            use tokio::task;
            
            let pool = Arc::new(create_test_pool().await?);
            let mut handles = vec![];
            
            // Spawn multiple tasks that each get a connection
            for _ in 0..5 {
                let pool = Arc::clone(&pool);
                let handle = task::spawn(async move {
                    let mut conn = pool.get().unwrap();
                    conn.test_connection().unwrap();
                });
                handles.push(handle);
            }
            
            // Wait for all tasks to complete
            for handle in handles {
                handle.await.unwrap();
            }
            
            Ok(())
        })
    }
    
    #[test]
    fn test_connection_timeout() -> Result<()> {
        run_async_test(|| async {
            use std::time::Duration;
            use tokio::time::timeout;
            
            // Create a pool with a very short timeout
            let database_url = if cfg!(feature = "sqlite") {
                "file:connection_timeout_test?mode=memory&cache=shared"
            } else if cfg!(feature = "postgres") {
                "postgres://postgres:postgres@localhost/connection_timeout_test"
            } else {
                return Err(Error::other("No database backend feature enabled"));
            };
            
            let pool = DbPool::new(database_url).await?;
            
            // Get all available connections to exhaust the pool
            let mut connections = vec![];
            for _ in 0..pool.size() {
                connections.push(pool.get_async().await?);
            }
            
            // Try to get another connection, which should time out
            let result = timeout(Duration::from_millis(100), pool.get_async()).await;
            
            // Clean up
            drop(connections);
            
            // Verify that we got a timeout error
            assert!(result.is_err(), "Should have timed out when getting a connection");
            
            // Test that we can get a connection after dropping one
            if let Some(conn) = connections.pop() {
                drop(conn);
                
                // Now we should be able to get a connection again
                let conn = pool.get_async().await;
                assert!(conn.is_ok(), "Should be able to get connection after previous was dropped");
            }
            
            Ok(())
        })
    }
}