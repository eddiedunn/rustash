//! Database connection management

use crate::error::Result;
use diesel::prelude::*;
use std::env;

#[cfg(feature = "sqlite")]
use diesel::sqlite::SqliteConnection;

#[cfg(feature = "postgres")]
use diesel::pg::PgConnection;

/// Database connection type based on feature flags
#[cfg(feature = "sqlite")]
pub type DbConnection = SqliteConnection;

#[cfg(feature = "postgres")]
pub type DbConnection = PgConnection;

/// Establish a database connection
pub fn establish_connection() -> Result<DbConnection> {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        #[cfg(feature = "sqlite")]
        return "rustash.db".to_string();
        
        #[cfg(feature = "postgres")]
        return "postgres://localhost/rustash".to_string();
    });

    #[cfg(feature = "sqlite")]
    {
        let conn = SqliteConnection::establish(&database_url)?;
        Ok(conn)
    }

    #[cfg(feature = "postgres")]
    {
        let conn = PgConnection::establish(&database_url)?;
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