//! Placeholder Knowledge Graph service implementation

use crate::storage::StorageBackend;
use std::sync::Arc;

#[derive(Debug)]
pub struct KnowledgeGraphService {
    backend: Arc<Box<dyn StorageBackend>>,
}

impl KnowledgeGraphService {
    pub fn new(backend: Arc<Box<dyn StorageBackend>>) -> Self {
        Self { backend }
    }
}
