//! Core memory item trait for Rustash storage system.

use chrono::{DateTime, Utc};
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/// The core trait for any piece of information stored in Rustash.
pub trait MemoryItem: erased_serde::Serialize + Send + Sync + fmt::Debug + Any + 'static {
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

    /// Create a boxed clone of this memory item
    fn clone_dyn(&self) -> Box<dyn MemoryItem>;

    /// Create a boxed clone of this memory item with Send + Sync bounds
    fn clone_dyn_send_sync(&self) -> Box<dyn MemoryItem + Send + Sync>;

    /// Returns a reference to the Any trait to allow for downcasting
    fn as_any(&self) -> &dyn Any;
}

// This allows us to serialize a `Box<dyn MemoryItem>`
erased_serde::serialize_trait_object!(MemoryItem);

// Implement MemoryItem for Box<dyn MemoryItem>
impl MemoryItem for Box<dyn MemoryItem> {
    fn id(&self) -> Uuid {
        (**self).id()
    }

    fn item_type(&self) -> &'static str {
        (**self).item_type()
    }

    fn content(&self) -> &str {
        (**self).content()
    }

    fn metadata(&self) -> HashMap<String, Value> {
        (**self).metadata()
    }

    fn created_at(&self) -> DateTime<Utc> {
        (**self).created_at()
    }

    fn updated_at(&self) -> DateTime<Utc> {
        (**self).updated_at()
    }

    fn clone_dyn(&self) -> Box<dyn MemoryItem> {
        (**self).clone_dyn()
    }

    fn clone_dyn_send_sync(&self) -> Box<dyn MemoryItem + Send + Sync> {
        (**self).clone_dyn_send_sync()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        (**self).as_any()
    }
}

// Implement Clone for Box<dyn MemoryItem>
impl Clone for Box<dyn MemoryItem> {
    fn clone(&self) -> Self {
        self.clone_dyn()
    }
}

// Implement Clone for Box<dyn MemoryItem + Send + Sync>
impl Clone for Box<dyn MemoryItem + Send + Sync> {
    fn clone(&self) -> Self {
        (**self).clone_dyn_send_sync()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, serde::Serialize)]
    struct TestMemory {
        id: Uuid,
        content: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    impl MemoryItem for TestMemory {
        fn id(&self) -> Uuid {
            self.id
        }
        fn item_type(&self) -> &'static str {
            "test"
        }
        fn content(&self) -> &str {
            &self.content
        }
        fn metadata(&self) -> HashMap<String, Value> {
            HashMap::new()
        }
        fn created_at(&self) -> DateTime<Utc> {
            self.created_at
        }
        fn updated_at(&self) -> DateTime<Utc> {
            self.updated_at
        }

        fn clone_dyn(&self) -> Box<dyn MemoryItem> {
            Box::new(self.clone())
        }

        fn clone_dyn_send_sync(&self) -> Box<dyn MemoryItem + Send + Sync> {
            Box::new(self.clone())
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
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
