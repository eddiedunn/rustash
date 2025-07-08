//! SQLite backend implementation for Rustash storage.

use super::StorageBackend;
use crate::{
    error::{Error, Result},
    models::{DbSnippet, NewDbSnippet, Snippet},
};
use async_trait::async_trait;
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, Pool, PooledConnection},
    sqlite::SqliteConnection,
};
use std::sync::Arc;
use uuid::Uuid;

/// A SQLite-backed storage implementation.
#[derive(Debug, Clone)]
pub struct SqliteBackend {
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
}

impl SqliteBackend {
    /// Create a new SQLite backend with the given connection pool.
    pub fn new(pool: Pool<ConnectionManager<SqliteConnection>>) -> Self {
        Self { pool: Arc::new(pool) }
    }

    /// Get a connection from the pool.
    fn get_conn(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<SqliteConnection>>> {
        self.pool.get().map_err(Error::from)
    }
}

#[async_trait]
impl StorageBackend for SqliteBackend {
    async fn save(&self, item: &(dyn crate::memory::MemoryItem + Send + Sync)) -> Result<()> {
        use crate::schema::snippets::dsl::*;
        
        let snippet = item
            .as_any()
            .downcast_ref::<Snippet>()
            .ok_or_else(|| Error::other("Expected a Snippet"))?;

        let new_snippet: NewDbSnippet = snippet.clone().into();
        let mut conn = self.get_conn()?;

        // Use the table name directly since we're using the dsl
        diesel::insert_into(snippets)
            .values(&new_snippet)
            .execute(&mut *conn)?;

        Ok(())
    }

    async fn get(&self, id: &Uuid) -> Result<Option<Box<dyn crate::memory::MemoryItem + Send + Sync>>> {
        use crate::schema::snippets::dsl::*;
        
        let mut conn = self.get_conn()?;
        let snippet_uuid = id.to_string();
        
        let db_snippet = snippets
            .filter(uuid.eq(&snippet_uuid))
            .first::<DbSnippet>(&mut *conn)
            .optional()?;

        Ok(db_snippet.map(|s| {
            let snippet: Snippet = s.into();
            Box::new(snippet) as Box<dyn crate::memory::MemoryItem + Send + Sync>
        }))
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        use crate::schema::snippets::dsl::*;
        
        let mut conn = self.get_conn()?;
        let snippet_uuid = id.to_string();
        
        diesel::delete(snippets.filter(uuid.eq(snippet_uuid))).execute(&mut *conn)?;
        
        Ok(())
    }

    // Note: The query method was removed as it's not part of the StorageBackend trait

    async fn vector_search(
        &self,
        _embedding: &[f32],
        _limit: usize,
    ) -> Result<Vec<(Box<dyn crate::memory::MemoryItem + Send + Sync>, f32)>> {
        // TODO: Implement vector search for SQLite
        Ok(Vec::new())
    }
    
    async fn add_relation(
        &self,
        _from: &Uuid,
        _to: &Uuid,
        _relation_type: &str,
    ) -> Result<()> {
        // SQLite doesn't natively support graph relationships
        // This would require additional tables and logic
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        database::create_test_pool,
        models::{Query, Snippet},
    };
    use chrono::Utc;
    use uuid::Uuid;

    async fn create_test_backend() -> SqliteBackend {
        let pool = create_test_pool();
        let backend = SqliteBackend::new(pool);
        
        // Run migrations
        let mut conn = backend.get_conn().unwrap();
        diesel_migrations::run_pending_migrations(&mut *conn).unwrap();
        
        backend
    }

    #[tokio::test]
    async fn test_save_and_get() {
        let backend = create_test_backend().await;
        
        let snippet = Snippet {
            id: Uuid::new_v4(),
            title: "Test Snippet".to_string(),
            content: "Test content".to_string(),
            tags: vec!["test".to_string()],
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        // Save the snippet
        backend.save(&snippet).await.unwrap();
        
        // Retrieve it
        let retrieved = backend.get(&snippet.id).await.unwrap().unwrap();
        let retrieved_snippet = retrieved
            .as_any()
            .downcast_ref::<Snippet>()
            .unwrap();
            
        assert_eq!(retrieved_snippet.title, "Test Snippet");
        assert_eq!(retrieved_snippet.content, "Test content");
        assert_eq!(retrieved_snippet.tags, vec!["test"]);
    }
    
    #[tokio::test]
    async fn test_query() {
        let backend = create_test_backend().await;
        
        // Clear any existing test data
        let mut conn = backend.get_conn().unwrap();
        diesel::delete(crate::schema::snippets::table)
            .execute(&mut *conn)
            .unwrap();
        
        // Add test snippets
        let snippets = vec![
            Snippet {
                id: Uuid::new_v4(),
                title: "Rust Ownership".to_string(),
                content: "Ownership is a set of rules that govern how a Rust program manages memory.".to_string(),
                tags: vec!["rust".to_string(), "memory".to_string()],
                embedding: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            Snippet {
                id: Uuid::new_v4(),
                title: "Python Lists".to_string(),
                content: "Lists are mutable sequences, typically used to store collections of homogeneous items.".to_string(),
                tags: vec!["python".to_string(), "data-structures".to_string()],
                embedding: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];
        
        for snippet in &snippets {
            backend.save(snippet).await.unwrap();
        }
        
        // Test text search
        let query = Query {
            text_filter: Some("Rust".to_string()),
            tag_filter: None,
            limit: 10,
        };
        
        let results = backend.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        
        // Test tag filter
        let query = Query {
            text_filter: None,
            tag_filter: Some("python".to_string()),
            limit: 10,
        };
        
        let results = backend.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
    }
    
    #[tokio::test]
    async fn test_delete() {
        let backend = create_test_backend().await;
        
        let snippet = Snippet {
            id: Uuid::new_v4(),
            title: "To be deleted".to_string(),
            content: "This will be deleted".to_string(),
            tags: vec!["test".to_string()],
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        // Save the snippet
        backend.save(&snippet).await.unwrap();
        
        // Verify it exists
        assert!(backend.get(&snippet.id).await.unwrap().is_some());
        
        // Delete it
        backend.delete(&snippet.id).await.unwrap();
        
        // Verify it's gone
        assert!(backend.get(&snippet.id).await.unwrap().is_none());
    }
}
