//! Database connection management

use crate::error::{Error, Result};
use diesel::prelude::*;
use std::path::{Path, PathBuf};
use std::env;
use std::ffi::OsStr;
use home::home_dir;

#[cfg(feature = "sqlite")]
use diesel::sqlite::SqliteConnection;

#[cfg(feature = "postgres")]
use diesel::pg::PgConnection;

/// Database connection type based on feature flags
#[cfg(feature = "sqlite")]
pub type DbConnection = SqliteConnection;

#[cfg(feature = "postgres")]
pub type DbConnection = PgConnection;

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
    
    // Check if parent directory exists and is writable
    if !parent.exists() {
        return Err(Error::other("Database directory does not exist"));
    }
    
    let metadata = std::fs::metadata(parent).map_err(|e| {
        Error::other(format!("Cannot access database directory: {}", e))
    })?;
    
    if !metadata.is_dir() {
        return Err(Error::other("Database path must be a directory"));
    }
    
    // Check write permissions
    let test_file = parent.join(".rustash_write_test");
    std::fs::write(&test_file, "").map_err(|e| {
        Error::other(format!("Cannot write to database directory: {}", e))
    })?;
    let _ = std::fs::remove_file(test_file);
    
    Ok(())
}

/// Establish a database connection
pub fn establish_connection() -> Result<DbConnection> {
    let database_url = match env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            // Use default path if DATABASE_URL is not set
            let default_path = default_db_path()?;
            default_path.to_string_lossy().into_owned()
        }
    };

    #[cfg(feature = "sqlite")]
    {
        // For SQLite, validate the file path
        if let Ok(path) = std::path::Path::new(&database_url).canonicalize() {
            validate_db_path(&path)?;
            
            // Ensure the parent directory exists
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        Error::other(format!("Failed to create database directory: {}", e))
                    })?;
                }
            }
        }
        
        let mut conn = SqliteConnection::establish(&database_url).map_err(|e| {
            Error::other(format!("Failed to connect to database at '{}': {}", database_url, e))
        })?;
        
        // Enable foreign key support for SQLite
        diesel::sql_query("PRAGMA foreign_keys = ON")
            .execute(&mut conn)
            .map_err(|e| Error::other(format!("Failed to enable foreign keys: {}", e)))?;
            
        Ok(conn)
    }

    #[cfg(feature = "postgres")]
    {
        // For PostgreSQL, validate the connection string format
        if !database_url.starts_with("postgres://") && !database_url.starts_with("postgresql://") {
            return Err(Error::other("PostgreSQL connection string must start with 'postgres://' or 'postgresql://'"));
        }
        
        let conn = PgConnection::establish(&database_url).map_err(|e| {
            Error::other(format!("Failed to connect to PostgreSQL database: {}", e))
        })?;
        
        Ok(conn)
    }
}

/// Establish a test database connection (in-memory SQLite)
#[cfg(test)]
pub fn establish_test_connection() -> Result<DbConnection> {
    #[cfg(feature = "sqlite")]
    {
        let mut conn = SqliteConnection::establish(":memory:")?;
        
        // Run migrations for test database
        use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
        const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|e| crate::error::Error::other(format!("Migration error: {}", e)))?;
        
        Ok(conn)
    }
    
    #[cfg(feature = "postgres")]
    {
        // For PostgreSQL tests, use a test database
        let database_url = env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost/rustash_test".to_string());
        let conn = PgConnection::establish(&database_url)?;
        Ok(conn)
    }
}