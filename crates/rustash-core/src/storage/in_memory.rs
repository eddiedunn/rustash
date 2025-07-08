//! In-memory storage backend for Rustash.

use crate::error::Result;
use crate::memory::MemoryItem;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

/// A trait defining the contract for storage backends.
/// This allows for interchangeable storage systems (SQLite, Postgres, etc.).
#[async_trait]
pub trait StorageBackend: Send + Sync + std::fmt::Debug {
    /// Save a memory item to the storage.
    async fn save(&self, item: &(dyn MemoryItem + Send + Sync)) -> Result<()>;

    /// Retrieve a memory item by its ID.
    async fn get(&self, id: &Uuid) -> Result<Option<Box<dyn MemoryItem + Send + Sync>>>;

    /// Delete a memory item by its ID.
    async fn delete(&self, id: &Uuid) -> Result<()>;

    /// Perform a vector similarity search.
    async fn vector_search(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(Box<dyn MemoryItem + Send + Sync>, f32)>>;

    /// Add a relationship between two memory items (for graph capabilities).
    async fn add_relation(
        &self,
        from: &Uuid,
        to: &Uuid,
        relation_type: &str,
    ) -> Result<()>;
}

/// A simple in-memory implementation for testing and development.
#[derive(Debug, Default)]
pub struct InMemoryBackend {
    items: RwLock<HashMap<Uuid, Box<dyn MemoryItem + Send + Sync>>>,
}

#[async_trait]
impl StorageBackend for InMemoryBackend {
    async fn save(&self, item: &(dyn MemoryItem + Send + Sync)) -> Result<()> {
        let mut items = self.items.write().unwrap();
        items.insert(item.id(), item.clone_dyn_send_sync());
        Ok(())
    }

    async fn get(&self, id: &Uuid) -> Result<Option<Box<dyn MemoryItem + Send + Sync>>> {
        let items = self.items.read().unwrap();
        Ok(items.get(id).map(|item| item.clone_dyn_send_sync()))
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        let mut items = self.items.write().unwrap();
        items.remove(id);
        Ok(())
    }

    async fn vector_search(
        &self,
        _embedding: &[f32],
        _limit: usize,
    ) -> Result<Vec<(Box<dyn MemoryItem + Send + Sync>, f32)>> {
        // Simple implementation that just returns all items with a score of 1.0
        let items = self.items.read().unwrap();
        let results = items
            .values()
            .map(|item| (item.clone_dyn_send_sync(), 1.0))
            .collect();
        Ok(results)
    }

    async fn add_relation(
        &self,
        _from: &Uuid,
        _to: &Uuid,
        _relation_type: &str,
    ) -> Result<()> {
        // In-memory implementation doesn't support relations
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;

    #[derive(Debug, Clone)]
    struct TestMemory {
        id: Uuid,
        content: String,
        created_at: chrono::DateTime<Utc>,
        updated_at: chrono::DateTime<Utc>,
    }

    impl MemoryItem for TestMemory {
        fn id(&self) -> Uuid { self.id }
        fn item_type(&self) -> &'static str { "test" }
        fn content(&self) -> &str { &self.content }
        fn metadata(&self) -> HashMap<String, serde_json::Value> { HashMap::new() }
        fn created_at(&self) -> chrono::DateTime<Utc> { self.created_at }
        fn updated_at(&self) -> chrono::DateTime<Utc> { self.updated_at }
        
        fn clone_dyn(&self) -> Box<dyn MemoryItem> {
            Box::new(self.clone())
        }
    }
    
    impl TestMemory {
        fn new(content: &str) -> Self {
            let now = Utc::now();
            Self {
                id: Uuid::new_v4(),
                content: content.to_string(),
                created_at: now,
                updated_at: now,
            }
        }
    }

    #[tokio::test]
    async fn test_in_memory_backend() {
        let backend = InMemoryBackend::default();
        let test_item = TestMemory::new("test content");
        let test_id = test_item.id;
        
        // Test save
        backend.save(&test_item).await.unwrap();
        
        // Test get
        let retrieved = backend.get(&test_id).await.unwrap().unwrap();
        assert_eq!(retrieved.id(), test_id);
        assert_eq!(retrieved.content(), "test content");
        
        // Test vector search (dummy implementation, just checks it doesn't panic)
        let results = backend.vector_search(&[], 10).await.unwrap();
        assert!(!results.is_empty());
        
        // Test delete
        backend.delete(&test_id).await.unwrap();
        assert!(backend.get(&test_id).await.unwrap().is_none());
        
        // Test clone
        let test_item2 = TestMemory::new("another test");
        let test_id2 = test_item2.id;
        backend.save(&test_item2).await.unwrap();
        
        let retrieved2 = backend.get(&test_id2).await.unwrap().unwrap();
        assert_eq!(retrieved2.id(), test_id2);
    }
}
