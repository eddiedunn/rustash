//! Placeholder RAG service implementation

use crate::storage::StorageBackend;
use std::sync::Arc;

#[derive(Debug)]
pub struct RAGService {
    backend: Arc<Box<dyn StorageBackend>>,
}

impl RAGService {
    pub fn new(backend: Arc<Box<dyn StorageBackend>>) -> Self {
        Self { backend }
    }
}
