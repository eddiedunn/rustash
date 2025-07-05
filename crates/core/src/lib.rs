//! # Rustash Core Library
//!
//! Core functionality for the Rustash snippet manager.
//! Provides database operations, snippet management, and search capabilities.

#![deny(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

pub mod database;
pub mod models;
pub mod schema;
pub mod services;
pub mod search;
pub mod clipboard;
pub mod placeholders;

pub use database::Database;
pub use models::*;
pub use services::*;

/// Result type used throughout the library
pub type Result<T> = anyhow::Result<T>;

/// Error type for Rustash operations
#[derive(thiserror::Error, Debug)]
pub enum RustashError {
    /// Database operation failed
    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),
    
    /// Connection pool error
    #[error("Connection pool error: {0}")]
    Pool(#[from] r2d2::Error),
    
    /// Search index error
    #[error("Search error: {0}")]
    Search(#[from] tantivy::TantivyError),
    
    /// Clipboard operation failed
    #[error("Clipboard error: {0}")]
    Clipboard(String),
    
    /// Placeholder expansion failed
    #[error("Placeholder error: {0}")]
    Placeholder(String),
    
    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),
}

/// Configuration for the Rustash application
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Config {
    /// Database connection URL
    pub database_url: String,
    /// Enable vector search
    pub vector_search: bool,
    /// Search index path
    pub search_index_path: String,
    /// Default category for new snippets
    pub default_category: String,
    /// Maximum number of search results
    pub max_search_results: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_url: "rustash.db".to_string(),
            vector_search: false,
            search_index_path: "search_index".to_string(),
            default_category: "general".to_string(),
            max_search_results: 50,
        }
    }
}

/// Initialize the Rustash library with configuration
pub fn init(config: Config) -> Result<()> {
    // Setup logging, database, search index, etc.
    Ok(())
}

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");