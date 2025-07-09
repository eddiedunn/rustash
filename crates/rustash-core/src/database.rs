use crate::error::{Error, Result};
use diesel_async::pooled_connection::{bb8::Pool, AsyncDieselConnectionManager, bb8::PooledConnection};
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use std::path::{Path, PathBuf};
use home::home_dir;

#[cfg(feature = "sqlite")]
pub type AsyncDbConnection = diesel_async::AsyncSqliteConnection;
#[cfg(feature = "postgres")]
pub type AsyncDbConnection = diesel_async::AsyncPgConnection;

pub type DbPool = Pool<AsyncDbConnection>;
pub type DbConnection = AsyncConnectionWrapper<AsyncDbConnection>;

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

pub async fn create_pool(database_url: &str) -> Result<DbPool> {
    let config = AsyncDieselConnectionManager::<AsyncDbConnection>::new(database_url);
    Pool::builder()
        .build(config)
        .await
        .map_err(|e| Error::other(format!("Failed to create connection pool: {e}")))
}

pub async fn create_connection_pool() -> Result<DbPool> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        default_db_path()
            .expect("Failed to get default database path")
            .to_str()
            .expect("Invalid UTF-8 in path")
            .to_string()
    });

    if database_url.starts_with("file:") || !database_url.contains(':') {
        validate_db_path(Path::new(&database_url))?;
    }

    create_pool(&database_url).await
}

pub async fn create_test_pool() -> Result<DbPool> {
    #[cfg(feature = "sqlite")]
    {
        return create_pool("sqlite::memory:").await;
    }
    #[cfg(feature = "postgres")]
    {
        let url = "postgres://postgres:postgres@localhost/postgres";
        return create_pool(url).await;
    }
    #[allow(unreachable_code)]
    Err(Error::other("No database backend feature enabled"))
}
