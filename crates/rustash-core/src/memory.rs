//! Core memory item trait for Rustash storage system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde_json::Value;

/// The core trait for any piece of information stored in Rustash.
pub trait MemoryItem: erased_serde::Serialize + Send + Sync + std::fmt::Debug {
    /// Returns the unique identifier for this memory item
    fn id(&self) -> Uuid;
    
    /// Returns the type of this memory item as a static string
    fn item_type(&self) -> &'static str;
    
    /// Returns the main content of this memory item
    fn content(&self) -> &str;
    
    /// Returns a map of metadata associated with this memory item
    fn metadata(&self) -> HashMap<String, Value>;
    
    /// Returns when this memory item was created
    fn created_at(&self) -> DateTime<Utc>;
    
    /// Returns when this memory item was last updated
    fn updated_at(&self) -> DateTime<Utc>;
}

// This allows us to serialize a `Box<dyn MemoryItem>`
erased_serde::serialize_trait_object!(MemoryItem);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Debug, Serialize, Deserialize)]
    struct TestMemory {
        id: Uuid,
        content: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    impl MemoryItem for TestMemory {
        fn id(&self) -> Uuid { self.id }
        fn item_type(&self) -> &'static str { "test" }
        fn content(&self) -> &str { &self.content }
        fn metadata(&self) -> HashMap<String, Value> { HashMap::new() }
        fn created_at(&self) -> DateTime<Utc> { self.created_at }
        fn updated_at(&self) -> DateTime<Utc> { self.updated_at }
    }

    #[test]
    fn test_memory_item_trait() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let test_item = TestMemory {
            id,
            content: "Test content".to_string(),
            created_at: now,
            updated_at: now,
        };

        assert_eq!(test_item.id(), id);
        assert_eq!(test_item.item_type(), "test");
        assert_eq!(test_item.content(), "Test content");
        assert!(test_item.metadata().is_empty());
        assert_eq!(test_item.created_at(), now);
        assert_eq!(test_item.updated_at(), now);
    }
}
