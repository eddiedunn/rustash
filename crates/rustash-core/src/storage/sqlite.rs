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
        
        // Try to downcast to SnippetWithTags first
        if let Some(snippet_with_tags) = item.as_any().downcast_ref::<crate::models::SnippetWithTags>() {
            let snippet = Snippet {
                uuid: snippet_with_tags.uuid.clone(),
                title: snippet_with_tags.title.clone(),
                content: snippet_with_tags.content.clone(),
                tags: serde_json::to_string(&snippet_with_tags.tags).unwrap_or_else(|_| "[]".to_string()),
                embedding: snippet_with_tags.embedding.clone(),
                created_at: snippet_with_tags.created_at.naive_utc(),
                updated_at: snippet_with_tags.updated_at.naive_utc(),
            };
            let new_snippet: NewDbSnippet = snippet.into();
            let mut conn = self.get_conn()?;
            
            diesel::insert_into(snippets)
                .values(&new_snippet)
                .execute(&mut *conn)?;
                
            return Ok(());
        }
        
        // Fall back to Snippet
        if let Some(snippet) = item.as_any().downcast_ref::<Snippet>() {
            let new_snippet: NewDbSnippet = snippet.clone().into();
            let mut conn = self.get_conn()?;
            
            diesel::insert_into(snippets)
                .values(&new_snippet)
                .execute(&mut *conn)?;
                
            return Ok(());
        }
        
        Err(Error::other("Expected a Snippet or SnippetWithTags"))
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
            let snippet_with_tags: crate::models::SnippetWithTags = snippet.into();
            Box::new(snippet_with_tags) as Box<dyn crate::memory::MemoryItem + Send + Sync>
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
        models::{Snippet, SnippetWithTags},
    };
    use chrono::Utc;
    use diesel_migrations::{embed_migrations, MigrationHarness};
    use serde_json;
    use uuid::Uuid;

    // This will embed the migrations in the binary
    pub const MIGRATIONS: diesel_migrations::EmbeddedMigrations = embed_migrations!("migrations");

    async fn create_test_backend() -> SqliteBackend {
        // Create a test pool with an in-memory SQLite database
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder()
            .max_size(1) // We only need one connection for testing
            .build(manager)
            .expect("Failed to create test pool");
        
        // Get a connection from the pool to run migrations
        let mut conn = pool.get().expect("Failed to get connection from pool");
        
        // Run migrations on the same connection that will be used by the tests
        conn.run_pending_migrations(MIGRATIONS).expect("Failed to run migrations");
        
        // Create the backend with the same pool
        SqliteBackend::new(pool)
    }

    #[tokio::test]
    async fn test_save_and_get() {
        let backend = create_test_backend().await;
        
        let snippet_id = Uuid::new_v4();
        let snippet = Snippet {
            uuid: snippet_id.to_string(),
            title: "Test Snippet".to_string(),
            content: "Test content".to_string(),
            tags: serde_json::to_string(&vec!["test".to_string()]).unwrap(),
            embedding: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };
        
        // Convert Snippet to a type that implements MemoryItem
        let snippet_with_tags = SnippetWithTags::from(snippet.clone());
        
        // Save the snippet
        backend.save(&snippet_with_tags).await.unwrap();
        
        // Retrieve it
        let retrieved = backend.get(&snippet_id).await.unwrap().unwrap();
        let retrieved_snippet = retrieved
            .as_any()
            .downcast_ref::<SnippetWithTags>()
            .unwrap();
            
        assert_eq!(retrieved_snippet.title, "Test Snippet");
        assert_eq!(retrieved_snippet.content, "Test content");
        assert_eq!(retrieved_snippet.tags, vec!["test".to_string()]);
    }
    
    // Removed test_query as it relies on the non-existent query method
    
    #[tokio::test]
    async fn test_delete() {
        let backend = create_test_backend().await;
        
        let snippet_id = Uuid::new_v4();
        let snippet = Snippet {
            uuid: snippet_id.to_string(),
            title: "To be deleted".to_string(),
            content: "This will be deleted".to_string(),
            tags: serde_json::to_string(&vec!["test".to_string()]).unwrap(),
            embedding: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };
        
        // Save the snippet
        backend.save(&snippet).await.unwrap();
        
        // Verify it exists
        assert!(backend.get(&snippet_id).await.unwrap().is_some());
        
        // Delete it
        backend.delete(&snippet_id).await.unwrap();
        
        // Verify it's gone
        assert!(backend.get(&snippet_id).await.unwrap().is_none());
    }
}
