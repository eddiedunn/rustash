// crates/rustash-core/src/stash.rs

use crate::storage::StorageBackend;
use crate::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ServiceType {
    Snippet,
    RAG,
    KnowledgeGraph,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StashConfig {
    pub service_type: ServiceType,
    pub database_url: String,
}

/// Represents a live, initialized Stash with a name, config, and active backend.
pub struct Stash {
    pub name: String,
    pub config: StashConfig,
    pub backend: Box<dyn StorageBackend>,
}

impl Stash {
    /// Creates a new, initialized Stash by setting up its backend.
    pub async fn new(name: &str, config: StashConfig) -> Result<Self> {
        let backend = crate::create_backend(&config.database_url).await?;
        Ok(Self {
            name: name.to_string(),
            config,
            backend,
        })
    }
}
