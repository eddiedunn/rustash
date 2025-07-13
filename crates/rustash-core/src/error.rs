//! Error handling for Rustash Core

use std::fmt;
use thiserror::Error;
use uuid::Uuid;

#[cfg(feature = "sqlite")]
use r2d2;

// Don't derive From for tokio_postgres::Error to avoid conflict with manual implementation
#[cfg(feature = "postgres")]
use tokio_postgres::error::Error as PgError;

#[cfg(feature = "bb8")]
use bb8::RunError as Bb8RunError;

/// Result type alias for Rustash operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for Rustash Core operations
#[derive(Error, Debug)]
pub enum Error {
    /// Database operation errors
    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),

    /// Database connection errors
    #[error("Connection error: {0}")]
    Connection(#[from] diesel::ConnectionError),

    /// Connection pool errors
    #[error("Connection pool error: {0}")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sqlite")))]
    #[cfg(feature = "sqlite")]
    ConnectionPool(#[from] r2d2::Error),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Async runtime errors
    #[error("Runtime error: {0}")]
    Runtime(String),

    /// Bincode serialization/deserialization errors
    #[error("Bincode error: {0}")]
    Bincode(#[from] bincode::Error),

    /// Connection pool errors
    #[error("Connection pool error: {0}")]
    Pool(String),

    /// PostgreSQL errors
    #[error("PostgreSQL error: {0}")]
    #[cfg(feature = "postgres")]
    Postgres(#[source] PgError),

    /// UUID parsing errors
    #[error("Invalid UUID: {0}")]
    InvalidUuid(#[from] uuid::Error),

    /// Not found errors
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Duplicate entry errors
    #[error("Duplicate entry: {0}")]
    Duplicate(String),

    /// Permission/authorization errors
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Generic error for other cases
    #[error("Error: {0}")]
    Other(String),
}

/// Extension trait for converting Option<T> to Result<Error>
pub trait OptionExt<T> {
    /// Convert an Option to a Result, mapping None to Error::NotFound
    fn or_not_found(self, msg: impl AsRef<str>) -> Result<T>;
}

impl<T> OptionExt<T> for Option<T> {
    fn or_not_found(self, msg: impl AsRef<str>) -> Result<T> {
        self.ok_or_else(|| Error::NotFound(msg.as_ref().to_string()))
    }
}

/// Extension trait for converting UUID strings to Uuid with better error handling
pub trait UuidExt {
    /// Parse a string as a UUID, returning a Result
    fn parse_uuid(&self) -> Result<Uuid>;
}

impl UuidExt for str {
    fn parse_uuid(&self) -> Result<Uuid> {
        self.parse::<Uuid>().map_err(Error::InvalidUuid)
    }
}

impl UuidExt for String {
    fn parse_uuid(&self) -> Result<Uuid> {
        self.as_str().parse_uuid()
    }
}

impl Error {
    /// Create a validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    /// Create a not found error
    pub fn not_found(resource: impl fmt::Display) -> Self {
        Self::NotFound(format!("{} not found", resource))
    }

    /// Create a duplicate entry error
    pub fn duplicate(resource: impl fmt::Display) -> Self {
        Self::Duplicate(format!("{} already exists", resource))
    }

    /// Create a permission denied error
    pub fn permission_denied(action: impl fmt::Display) -> Self {
        Self::PermissionDenied(format!("Permission denied for: {}", action))
    }

    /// Create a generic error
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }

    /// Check if this is a not found error
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound(_))
    }

    /// Check if this is a duplicate entry error
    pub fn is_duplicate(&self) -> bool {
        matches!(self, Self::Duplicate(_))
    }

    /// Check if this is a permission denied error
    pub fn is_permission_denied(&self) -> bool {
        matches!(self, Self::PermissionDenied(_))
    }
}

#[cfg(feature = "postgres")]
impl From<tokio_postgres::Error> for Error {
    fn from(err: tokio_postgres::Error) -> Self {
        Error::Postgres(err)
    }
}

#[cfg(feature = "postgres")]
impl From<bb8::RunError<tokio_postgres::Error>> for Error {
    fn from(err: bb8::RunError<tokio_postgres::Error>) -> Self {
        match err {
            bb8::RunError::User(e) => Error::Postgres(e),
            bb8::RunError::TimedOut => Error::Pool("Connection timed out".into()),
        }
    }
}

#[cfg(feature = "bb8")]
impl<T: std::error::Error + 'static> From<bb8::RunError<T>> for Error {
    fn from(err: bb8::RunError<T>) -> Self {
        match err {
            bb8::RunError::User(e) => Error::Pool(format!("Connection error: {}", e)),
            bb8::RunError::TimedOut => Error::Pool("Connection timed out".into()),
        }
    }
}

#[cfg(feature = "sqlite")]
impl From<r2d2::Error> for Error {
    fn from(err: r2d2::Error) -> Self {
        Error::Pool(format!("Connection pool error: {}", err))
    }
}

impl From<bincode::Error> for Error {
    fn from(err: bincode::Error) -> Self {
        Error::Bincode(err)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Database(e) => Some(e),
            Error::Connection(e) => Some(e),
            #[cfg(feature = "sqlite")]
            Error::ConnectionPool(e) => Some(e),
            Error::Serialization(e) => Some(e),
            Error::Io(e) => Some(e),
            #[cfg(feature = "postgres")]
            Error::Postgres(e) => Some(e),
            Error::Bincode(e) => Some(e),
            _ => None,
        }
    }
}
