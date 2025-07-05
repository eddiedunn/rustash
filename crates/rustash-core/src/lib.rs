//! Rustash Core Library
//!
//! This crate provides the core functionality for the Rustash snippet manager,
//! including database operations, data models, and snippet management.

pub mod database;
pub mod models;
pub mod schema;
pub mod snippet;
pub mod error;

#[cfg(feature = "vector-search")]
pub mod search;

// Re-export commonly used types
pub use database::establish_connection;
pub use error::{Error, Result};
pub use models::{NewSnippet, Snippet, SnippetWithTags};
pub use snippet::{
    add_snippet, delete_snippet, expand_placeholders, get_snippet_by_id, list_snippets,
    list_snippets_with_tags, search_snippets, update_snippet,
};

#[cfg(feature = "vector-search")]
pub use search::search_similar_snippets;