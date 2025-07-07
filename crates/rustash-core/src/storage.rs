//! Storage backend trait definitions for Rustash.

use crate::memory::MemoryItem;
use crate::error::Result;
use async_trait::async_trait;
use uuid::Uuid;

/// A trait defining the contract for storage backends.
/// This allows for interchangeable storage systems (SQLite, Postgres, etc.).
#[async_trait]
pub trait StorageBackend: Send + Sync + std::fmt::Debug {
    /// Save a memory item to the storage.
    async fn save(&self, item: &dyn MemoryItem) -> Result<()>;

    /// Retrieve a memory item by its ID.
    async fn get(&self, id: &Uuid) -> Result<Option<Box<dyn MemoryItem>>>;

    /// Delete a memory item by its ID.
    async fn delete(&self, id: &Uuid) -> Result<()>;

    /// Perform a vector similarity search.
    async fn vector_search(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(Box<dyn MemoryItem>, f32)>>;

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
    items: std::sync::RwLock<std::collections::HashMap<Uuid, Box<dyn MemoryItem>>>,
}

#[async_trait]
impl StorageBackend for InMemoryBackend {
    async fn save(&self, item: &dyn MemoryItem) -> Result<()> {
        let mut items = self.items.write().unwrap();
        items.insert(item.id(), Box::from(item) as Box<dyn MemoryItem>);
        Ok(())
    }

    async fn get(&self, id: &Uuid) -> Result<Option<Box<dyn MemoryItem>>> {
        let items = self.items.read().unwrap();
        Ok(items.get(id).map(|item| item.clone_dyn()))
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
    ) -> Result<Vec<(Box<dyn MemoryItem>, f32)>> {
        // Simple implementation that returns all items with a dummy score
        let items = self.items.read().unwrap();
        let results = items
            .values()
            .map(|item| (item.clone_dyn(), 1.0))
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

// Helper trait to clone boxed MemoryItems
trait CloneDyn: MemoryItem {
    fn clone_dyn(&self) -> Box<dyn MemoryItem>;
}

impl<T> CloneDyn for T
where
    T: MemoryItem + Clone + 'static,
{
    fn clone_dyn(&self) -> Box<dyn MemoryItem> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryItem;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestMemory {
        id: Uuid,
        content: String,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: chrono::DateTime<chrono::Utc>,
    }

    impl MemoryItem for TestMemory {
        fn id(&self) -> Uuid { self.id }
        fn item_type(&self) -> &'static str { "test" }
        fn content(&self) -> &str { &self.content }
        fn metadata(&self) -> std::collections::HashMap<String, serde_json::Value> {
            std::collections::HashMap::new()
        }
        fn created_at(&self) -> chrono::DateTime<chrono::Utc> { self.created_at }
        fn updated_at(&self) -> chrono::DateTime<chrono::Utc> { self.updated_at }
    }

    #[tokio::test]
    async fn test_in_memory_backend() {
        let backend = InMemoryBackend::default();
        let now = chrono::Utc::now();
        let id = Uuid::new_v4();
        
        let test_item = TestMemory {
            id,
            content: "Test content".to_string(),
            created_at: now,
            updated_at: now,
        };

        // Test save
        backend.save(&test_item).await.unwrap();

        // Test get
        let retrieved = backend.get(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.id(), id);
        assert_eq!(retrieved.content(), "Test content");

        // Test vector search
        let results = backend.vector_search(&[0.0], 10).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].0.id(), id);

        // Test delete
        backend.delete(&id).await.unwrap();
        assert!(backend.get(&id).await.unwrap().is_none());
    }
}
