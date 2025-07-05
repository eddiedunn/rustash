//! Error handling for Rustash Core

use thiserror::Error;

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
    ConnectionPool(#[from] r2d2::Error),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error for other cases
    #[error("Error: {0}")]
    Other(String),
}

impl Error {
    /// Create a validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    /// Create a generic error
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}