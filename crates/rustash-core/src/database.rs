use crate::error::{Error, Result};
use std::path::PathBuf;
use std::sync::Arc;

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
            use diesel_async::{
                AsyncSqliteConnection,
                pooled_connection::AsyncDieselConnectionManager,
                async_connection_wrapper::AsyncConnectionWrapper,
            };
            use bb8::Pool as Bb8Pool;
            use std::convert::Infallible;
            use async_trait::async_trait;
            
            pub type Connection = AsyncSqliteConnection;
            pub type Manager = AsyncDieselConnectionManager<AsyncSqliteConnection>;
            pub type Pool = Bb8Pool<Manager>;
            pub type PooledConn = AsyncConnectionWrapper<AsyncSqliteConnection>;
            
            // Wrapper to provide a sync interface over async connection
            pub struct SyncConnectionWrapper(PooledConn);
            
            impl std::ops::Deref for SyncConnectionWrapper {
                type Target = AsyncSqliteConnection;
                
                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }
            
            impl std::ops::DerefMut for SyncConnectionWrapper {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.0
                }
            }
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
            
            impl std::ops::Deref for SyncConnectionWrapper {
                type Target = AsyncPgConnection;
                
                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }
            
            impl std::ops::DerefMut for SyncConnectionWrapper {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.0
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
pub type AsyncDbConnection = diesel_async::AsyncSqliteConnection;
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
        #[cfg(feature = "sqlite")]
        {
            // For SQLite, ensure the parent directory exists
            if database_url.starts_with("file:") || database_url.ends_with(".db") || database_url.ends_with(".sqlite") {
                let path = database_url.trim_start_matches("file:");
                if path != ":memory:" {
                    if let Some(parent) = Path::new(path).parent() {
                        if !parent.exists() {
                            std::fs::create_dir_all(parent).map_err(|e| {
                                Error::other(format!("Failed to create database directory: {}", e))
                            })?;
                        }
                    }
                }
            }
            
            let manager = AsyncDieselConnectionManager::<diesel_async::AsyncSqliteConnection>::new(database_url);
            let pool = Bb8Pool::builder()
                .max_size(16)
                .build(manager)
                .await
                .map_err(|e| Error::other(format!("Failed to create connection pool: {}", e)))?;
                
            Ok(Self(Arc::new(pool)))
        }
        
        #[cfg(feature = "postgres")]
        {
            use diesel_async::AsyncPgConnection;
            use diesel_async::pooled_connection::AsyncDieselConnectionManager;
            use bb8::Pool as Bb8Pool;
            
            let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
            let pool = Bb8Pool::builder()
                .build(manager)
                .await
                .map_err(|e| Error::other(format!("Failed to create connection pool: {}", e)))?;
                
            Ok(Self(Arc::new(pool)))
        }
    }

    /// Get a connection from the pool
    pub async fn get_async(&self) -> Result<DbConnection> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "sqlite")] {
                use diesel::SqliteConnection;
                
                // For SQLite, we need to use a blocking task
                let pool = self.0.downcast_ref::<Pool<ConnectionManager<SqliteConnection>>>()
                    .ok_or_else(|| Error::other("Failed to downcast SQLite connection pool"))?;
                
                // Get a connection from the pool
                let conn = pool.get()
                    .map_err(|e| Error::other(format!("Failed to get SQLite connection: {}", e)))?;
                
                Ok(conn.into())
            } else if #[cfg(feature = "postgres")] {
                let pool = self.0.downcast_ref::<bb8::Pool<AsyncDieselConnectionManager<diesel_async::AsyncPgConnection>>>()
                    .ok_or_else(|| Error::other("Failed to downcast PostgreSQL connection pool"))?;
                    
                let conn = pool.get_owned().await
                    .map_err(|e| Error::other(format!("Failed to get PostgreSQL connection: {}", e)))?;
                
                Ok(conn.into())
            } else {
                compile_error!("Either 'sqlite' or 'postgres' feature must be enabled");
            }
        }
    }

    /// Get a connection from the pool (synchronous interface)
    pub fn get(&self) -> Result<DbConnectionGuard> {
        // Create a new runtime for blocking on async operations
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Error::other(format!("Failed to create runtime: {}", e)))?;
        
        // Get a connection by blocking on the async operation
        let conn = rt.block_on(self.get_async())?;
        
        // Create a new connection guard
        Ok(DbConnectionGuard::new(conn, self.clone()))
    }
}
/// A wrapper around a pooled connection that implements `Deref` to the inner connection
pub struct DbConnectionGuard {
    #[cfg(feature = "sqlite")]
    conn: Box<dyn std::ops::Deref<Target = diesel_async::AsyncSqliteConnection> + Send + 'static>,
    #[cfg(feature = "postgres")]
    conn: Box<dyn std::ops::Deref<Target = diesel_async::AsyncPgConnection> + Send + 'static>,
    pool: DbPool,
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
    type Target = dyn std::ops::Deref<Target = DbConnection> + Send + 'static;
    
    fn deref(&self) -> &Self::Target {
        &*self.conn
    }
}

impl std::ops::DerefMut for DbConnectionGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.conn
    }
}

impl DbConnectionGuard {
    /// Create a new connection guard
    pub fn new(conn: DbConnection, pool: DbPool) -> Self {
        Self {
            conn: Box::new(conn),
            pool,
        }
    }
    
    /// Explicitly get the inner connection
    pub fn into_inner(self) -> DbConnection {
        *self.conn
    }
    
    /// Test if the database connection is still valid
    pub async fn test_connection(&mut self) -> Result<()> {
        use diesel::sql_query;
        
        // For SQLite
        #[cfg(feature = "sqlite")]
        {
            let _ = diesel_async::RunQueryDsl::execute(
                sql_query("SELECT 1"),
                &mut **self
            ).await?;
        }
            
        // For PostgreSQL
        #[cfg(feature = "postgres")]
        {
            let _ = diesel_async::RunQueryDsl::execute(
                sql_query("SELECT 1"),
                &mut **self
            ).await?;
        }
            
        Ok(())
    }
}

#[cfg(feature = "migrations")]
/// Run database migrations asynchronously
pub async fn run_migrations(pool: &DbPool) -> Result<()> {
    use diesel_migrations::{MigrationHarness, HarnessWithOutput};
    use std::path::Path;
    use std::sync::Arc;

    // Get a connection from the pool
    let mut conn = pool.get_async().await?;
    
    // Set up the migration harness
    let migrations_path = std::env::current_dir()?
        .parent()
        .ok_or_else(|| Error::other("Failed to get parent directory"))?
        .join("crates/rustash-core/migrations");
    
    // Run migrations
    let migration_result = tokio::task::spawn_blocking(move || {
        let mut harness = HarnessWithOutput::new(conn.inner_mut(), std::io::stdout());
        
        #[cfg(feature = "sqlite")]
        {
            use diesel::sqlite::Sqlite;
            diesel_migrations::FileBasedMigrations::from_path(&migrations_path)
                .map_err(|e| Error::other(format!("Failed to load migrations: {}", e)))?
                .run_pending_migrations(&mut harness)
                .map_err(|e| Error::other(format!("Failed to run migrations: {}", e)))?;
        }
        
        #[cfg(feature = "postgres")]
        {
            use diesel::pg::Pg;
            diesel_migrations::FileBasedMigrations::from_path(&migrations_path)
                .map_err(|e| Error::other(format!("Failed to load migrations: {}", e)))?
                .run_pending_migrations(&mut harness)
                .map_err(|e| Error::other(format!("Failed to run migrations: {}", e)))?;
        }
        
        Ok::<_, Error>(())
    }).await??;
    
    Ok(migration_result)
}

/// Default database filename
const DEFAULT_DB_FILENAME: &str = "rustash.db";

fn default_db_path() -> Result<PathBuf> {
    let home = home_dir().ok_or_else(|| Error::other("Could not determine home directory"))?;
    let path = home.join(".config").join("rustash").join(DEFAULT_DB_FILENAME);
    Ok(path)
}

fn validate_db_path(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        return Err(Error::other("Database path must be absolute"));
    }
    if path.is_dir() {
        return Err(Error::other("Database path cannot be a directory"));
    }
    Ok(())
}

/// Create a new database connection pool
pub async fn create_connection_pool() -> Result<DbPool> {
    use diesel_async::RunQueryDsl;
    
    // Get the database path from the environment or use the default
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => default_db_path()?.to_string_lossy().into_owned(),
    };
    
    // Validate the database path for SQLite
    if database_url.starts_with("file:") || database_url.ends_with(".db") || database_url.ends_with(".sqlite") {
        let path = Path::new(database_url.trim_start_matches("file:"));
        validate_db_path(path)?;
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
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
    let pool = DbPool::new(&database_url).await?;
    
    // Test the connection and enable foreign keys for SQLite
    {
        let mut conn = pool.get_async().await?;
        
        // Enable foreign key support for SQLite
        #[cfg(feature = "sqlite")]
        {
            diesel_async::RunQueryDsl::execute(
                diesel::sql_query("PRAGMA foreign_keys = ON"),
                &mut *conn
            )
            .await
            .map_err(|e| Error::other(format!("Failed to enable foreign keys: {}", e)))?;
                
            diesel_async::RunQueryDsl::execute(
                diesel::sql_query("PRAGMA journal_mode = WAL"),
                &mut *conn
            )
            .await
            .map_err(|e| Error::other(format!("Failed to enable WAL mode: {}", e)))?;
        }
    }
    
    Ok(pool)
}

/// For PostgreSQL, this creates a connection to a local test database.
/// The database name includes a unique number to ensure test isolation.
pub async fn create_test_pool() -> Result<DbPool> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use diesel::sql_query;
    use diesel_async::RunQueryDsl;

    static TEST_DB_NUM: AtomicUsize = AtomicUsize::new(0);
    let test_num = TEST_DB_NUM.fetch_add(1, Ordering::SeqCst);

    // For SQLite, use an in-memory database
    #[cfg(feature = "sqlite")]
    let database_url = format!("file:test_db_{}?mode=memory&cache=shared", test_num);

    // For PostgreSQL, use a test database with a unique name
    #[cfg(feature = "postgres")]
    let database_url = format!(
        "postgres://postgres:postgres@localhost/test_db_{}",
        test_num
    );

    // Create the connection pool to the test database
    let pool = DbPool::new(&database_url).await?;
    
    // Set up the database
    {
        let mut conn = pool.get_async().await?;
        
        // Database-specific setup
        if cfg!(feature = "sqlite") {
            // SQLite-specific setup - use block_in_place for synchronous operations
            let conn = &mut *conn;
            tokio::task::block_in_place(|| -> Result<()> {
                diesel::RunQueryDsl::execute(
                    diesel::sql_query("PRAGMA foreign_keys = ON"),
                    conn
                ).map_err(|e| Error::other(format!("Failed to enable foreign keys: {}")))?;
                
                diesel::RunQueryDsl::execute(
                    diesel::sql_query("PRAGMA journal_mode = WAL"),
                    conn
                ).map_err(|e| Error::other(format!("Failed to enable WAL mode: {}")))?;
                
                Ok(())
            })?;
        } else if cfg!(feature = "postgres") {
            // PostgreSQL-specific setup
            // Enable required extensions
            diesel_async::RunQueryDsl::execute(
                diesel::sql_query("CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\""),
                &mut *conn
            )
            .await
            .map_err(|e| Error::other(format!("Failed to create uuid-ossp extension: {}", e)))?;
                
            diesel_async::RunQueryDsl::execute(
                diesel::sql_query("CREATE EXTENSION IF NOT EXISTS \"pgcrypto\""),
                &mut *conn
            )
            .await
            .map_err(|e| Error::other(format!("Failed to create pgcrypto extension: {}", e)))?;
        }
        
        // Run migrations using Diesel's embedded migrations
        use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
        
        // This will include the migrations at compile time
        // The path is relative to the crate root (where Cargo.toml is located)
        const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
        
        // Run the migrations using spawn_blocking for SQLite since it's a blocking operation
        if cfg!(feature = "sqlite") {
            let migrations = MIGRATIONS;
            tokio::task::block_in_place(|| {
                conn.run_pending_migrations(migrations)
            })
            .map_err(|e| Error::other(format!("Failed to run migrations: {}", e)))?;
        } else {
            // For PostgreSQL, we can use async/await
            conn.run_pending_migrations(MIGRATIONS)
                .await
                .map_err(|e| Error::other(format!("Failed to run migrations: {}", e)))?;
        }
    }
    
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Arc;
    use tempfile::tempdir;
    use tokio::runtime::Runtime;
    use tokio::task;
    
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
    
    // Helper macro to create async test functions
    macro_rules! async_test {
        ($name:ident { $($body:tt)* }) => {
            #[test]
            fn $name() -> Result<()> {
                run_async_test(|| async { $($body)* })
            }
        };
    }
    
    async_test! { test_connection_pool {
        let pool = create_test_pool().await?;
        assert!(pool.get_async().await.is_ok(), "Should be able to get a connection from the pool");
        
        // Test that we can get multiple connections
        let conn1 = pool.get_async().await?;
        let conn2 = pool.get_async().await?;
        
        // Test that the connections are different
        if !is_postgres() {
            // For SQLite, we can compare the raw connection pointers
            let ptr1 = &*conn1 as *const _ as *const u8;
            let ptr2 = &*conn2 as *const _ as *const u8;
            assert_ne!(ptr1, ptr2, "Connections should be different instances");
        }
        
        Ok(())
    }}
    
    async_test! { test_connection_guard {
        let pool = create_test_pool().await?;
        let mut guard = pool.get_async().await?;
        
        // Test that we can use the guard
        assert!(guard.test_connection().await.is_ok(), "Connection test should succeed");
        
        // Test that the guard can be dereferenced
        let _: &dyn std::ops::Deref<Target = DbConnection> = &*guard;
        
        // Test that the guard can be converted back to a connection
        let _conn = guard.into_inner();
        
        Ok(())
    }}
    
    async_test! { test_default_db_path {
        // Test that we get a valid path
        let path = default_db_path()?;
        assert!(path.is_absolute(), "Default database path should be absolute");
        assert!(path.to_string_lossy().contains("rustash"), "Path should contain 'rustash'");
        
        // Test that the parent directory exists or can be created
        let parent = path.parent().unwrap();
        assert!(
            parent.exists() || fs::create_dir_all(parent).is_ok(),
            "Should be able to create parent directory if it doesn't exist"
        );
        
        // Test that the default filename is used
        assert_eq!(
            path.file_name().and_then(OsStr::to_str),
            Some(DEFAULT_DB_FILENAME),
            "Default filename should be used"
        );
        
        Ok(())
    }}
    
    async_test! { test_validate_db_path {
        // Test with a valid path
        let temp_dir = tempdir()?;
        let valid_path = temp_dir.path().join("test.db");
        validate_db_path(&valid_path)?;
        
        // Test with a path that's a directory
        let dir_path = temp_dir.path();
        assert!(
            validate_db_path(dir_path).is_err(), 
            "Should reject directory path"
        );
        
        // Test with a path that doesn't exist
        let non_existent_path = temp_dir.path().join("nonexistent/test.db");
        validate_db_path(&non_existent_path)?;
        
        // Test with a path outside the home directory (should be allowed)
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
            
            // Create a symlink pointing to the file
            let symlink_path = temp_dir.path().join("symlink.db");
            unix_fs::symlink("/etc/passwd", &symlink_path)?;
            
            // Test the actual validation
            let validation_result = validate_db_path(&symlink_path);
            assert!(
                validation_result.is_err(),
                "Should reject symlink paths for security"
            );
        }
        
        Ok(())
    }}
    
    async_test! { test_create_connection_pool {
        // Test with default configuration
        let pool = create_connection_pool().await?;
        let mut conn = pool.get_async().await?;
        
        // Test that we can execute a simple query
        #[cfg(feature = "postgres")]
        {
            let result: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1 + 0"))
                .get_result(&mut *conn)
                .await?;
            assert_eq!(result, 1, "Should be able to execute query on connection");
        }
        
        #[cfg(feature = "sqlite")]
        {
            let result: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1"))
                .get_result(&mut *conn)
                .await?;
            assert_eq!(result, 1, "Should be able to execute query on connection");
        }
        
        // Test with invalid database URL
        let invalid_url = if is_postgres() {
            "postgres://invalid:invalid@localhost/nonexistent"
        } else {
            "file:/nonexistent/path/test.db"
        };
        
        let result = {
            std::env::set_var("DATABASE_URL", invalid_url);
            let result = DbPool::new(invalid_url).await;
            std::env::remove_var("DATABASE_URL");
            result
        };
        
        assert!(result.is_err(), "Should fail with invalid database URL");
        
        Ok(())
    }}
    
    async_test! { test_connection_pool_multithreaded {
        let pool = Arc::new(create_test_pool().await?);
        let mut handles = vec![];
        
        // Spawn multiple tasks that each get a connection
        for _ in 0..5 {
            let pool = Arc::clone(&pool);
            let handle = task::spawn(async move {
                let mut conn = pool.get_async().await.unwrap();
                conn.test_connection().await.unwrap();
            });
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }
        
        Ok(())
    }}
    
    async_test! { test_connection_timeout {
        use std::time::Duration;
        use tokio::time::timeout;
        
        // Test with a very short timeout
        let pool = if is_postgres() {
            // For PostgreSQL, we can't easily test timeouts without a real server
            // So we'll just test that we can create a pool with a timeout
            DbPool::new("postgres://localhost:5432/test").await?
        } else {
            // For SQLite, we can test timeouts with an in-memory database
            DbPool::new("file::memory:?cache=shared").await?
        };
        
        // Test that we can get a connection within the timeout
        let result = timeout(
            Duration::from_secs(5),
            pool.get_async()
        ).await;
        
        // Verify we can get a connection
        assert!(result.is_ok(), "Should be able to get a connection within the timeout");
        
        // Get all available connections to exhaust the pool
        let mut connections = vec![];
        for _ in 0..pool.size() {
            connections.push(pool.get_async().await?);
        }
        
        // Try to get another connection, which should time out
        let result = timeout(Duration::from_millis(100), pool.get_async()).await;
        
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
    }}
}

// Add a test for the connection pool size
#[cfg(test)]
mod more_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_pool_size() -> Result<()> {
        let pool = create_test_pool().await?;
        assert!(pool.size() > 0, "Pool should have a positive size");
        Ok(())
    }
}