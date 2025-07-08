//! Storage backends for Rustash.

mod in_memory;
pub use in_memory::InMemoryBackend;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

// Re-export the StorageBackend trait
pub use crate::storage::in_memory::StorageBackend;
