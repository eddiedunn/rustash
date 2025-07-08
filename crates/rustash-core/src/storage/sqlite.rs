//! SQLite backend implementation for Rustash storage.

use super::StorageBackend;
use crate::{
    database::DbPool,
    error::{Error, Result},
    models::Snippet,
};
use async_trait::async_trait;
use diesel::{
    prelude::*,
    sql_query,
    sql_types::Text,
};
use diesel_async::{
    RunQueryDsl,
    AsyncConnection,
};
use std::sync::Arc;
use uuid::Uuid;

/// A SQLite-backed storage implementation.
#[derive(Debug, Clone)]
pub struct SqliteBackend {
    pool: Arc<DbPool>,
}

impl SqliteBackend {
    /// Create a new SQLite backend with the given connection pool.
    pub fn new(pool: DbPool) -> Self {
        Self { pool: Arc::new(pool) }
    }

    /// Get a connection from the pool.
    async fn get_conn(&self) -> Result<diesel_async::AsyncConnectionWrapper<diesel_async::AsyncSqliteConnection>> {
        self.pool.get_async().await
    }
    
    /// Convert a database row to a Snippet
    fn row_to_snippet(
        &self,
        row: diesel::sqlite::SqliteRow,
    ) -> Result<Snippet> {
        use diesel::row::NamedRow;
        
        let uuid: String = row.get("uuid").map_err(Error::from)?;
        let title: String = row.get("title").map_err(Error::from)?;
        let content: String = row.get("content").map_err(Error::from)?;
        let tags_json: String = row.get("tags").map_err(Error::from)?;
        let embedding: Option<Vec<u8>> = row.get("embedding").map_err(Error::from)?;
        let created_at: chrono::NaiveDateTime = row.get("created_at").map_err(Error::from)?;
        let updated_at: chrono::NaiveDateTime = row.get("updated_at").map_err(Error::from)?;
        
        // Validate the UUID format
        Uuid::parse_str(&uuid).map_err(Error::from)?;
        
        // Validate the tags JSON
        let _: Vec<String> = serde_json::from_str(&tags_json).map_err(Error::from)?;
        
        let snippet = Snippet {
            uuid,
            title,
            content,
            tags: tags_json,
            embedding,
            created_at,
            updated_at,
        };

        Ok(snippet)
    }
}

#[async_trait]
impl StorageBackend for SqliteBackend {
    async fn save(&self, item: &(dyn crate::memory::MemoryItem + Send + Sync)) -> Result<()> {
        use diesel::prelude::*;
        
        let snippet = item
            .as_any()
            .downcast_ref::<Snippet>()
            .ok_or_else(|| Error::other("Expected a Snippet"))?;

        // Validate the UUID format
        Uuid::parse_str(&snippet.uuid)
            .map_err(|e| Error::other(format!("Invalid UUID format: {}", e)))?;
            
        // Validate tags JSON
        let _: Vec<String> = serde_json::from_str(&snippet.tags)
            .map_err(|e| Error::other(format!("Invalid tags format: {}", e)))?;
            
        let now = chrono::Utc::now().naive_utc();
        
        let mut conn = self.get_conn().await?;
        
        // Use parameterized query to prevent SQL injection
        let query = r#"
            INSERT INTO snippets (uuid, title, content, tags, embedding, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (uuid) DO UPDATE
            SET title = excluded.title,
                content = excluded.content,
                tags = excluded.tags,
                embedding = excluded.embedding,
                updated_at = excluded.updated_at
        "#;
        
        sql_query(query)
            .bind::<Text, _>(&snippet.uuid)
            .bind::<Text, _>(&snippet.title)
            .bind::<Text, _>(&snippet.content)
            .bind::<Text, _>(&snippet.tags)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Binary>, _>(snippet.embedding.as_ref())
            .bind::<diesel::sql_types::Timestamp, _>(now)
            .bind::<diesel::sql_types::Timestamp, _>(now)
            .execute(&mut conn)
            .await
            .map_err(|e| Error::other(format!("Failed to save snippet: {}", e)))?;
    }

    async fn get(&self, id: &Uuid) -> Result<Option<Box<dyn crate::memory::MemoryItem + Send + Sync>>> {
        use diesel::prelude::*;
        
        let id_str = id.to_string();
        let mut conn = self.get_conn().await?;
        
        let query = "SELECT * FROM snippets WHERE uuid = ?";
        
        let result = sql_query(query)
            .bind::<Text, _>(&id_str)
            .get_result::<diesel::sqlite::SqliteRow>(&mut conn)
            .await;
            
        match result {
            Ok(row) => {
                let snippet = self.row_to_snippet(row)?;
                Ok(Some(Box::new(snippet)))
            },
            Err(diesel::result::Error::NotFound) => Ok(None),
            Err(e) => Err(Error::other(format!("Failed to get snippet: {}", e))),
        }
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        use diesel::prelude::*;
        
        let id_str = id.to_string();
        let mut conn = self.get_conn().await?;
        
        let query = "DELETE FROM snippets WHERE uuid = ?";
        
        sql_query(query)
            .bind::<Text, _>(&id_str)
            .execute(&mut conn)
            .await
            .map_err(|e| Error::other(format!("Failed to delete snippet: {}", e)))?;
            
        Ok(())
    }

    async fn vector_search(
        &self,
        _embedding: &[f32],
        _limit: usize,
    ) -> Result<Vec<(Box<dyn crate::memory::MemoryItem + Send + Sync>, f32)>> {
        // SQLite doesn't have built-in vector search capabilities
        // This is a placeholder implementation that returns an empty vector
        // In a real implementation, you might want to use an extension like SQLite VSS
        // or implement a simple cosine similarity function in SQL
        
        // For now, we'll just return an empty vector
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
    
    async fn query(
        &self,
        query: &crate::models::Query,
    ) -> Result<Vec<Box<dyn crate::memory::MemoryItem + Send + Sync>>> {
        use diesel::prelude::*;
        
        let mut conn = self.get_conn().await?;
        let mut results = Vec::new();
        
        // Start building the query
        let mut sql = "SELECT * FROM snippets WHERE 1=1".to_string();
        let mut params: Vec<Box<dyn diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite> + Send>> = Vec::new();
        
        // Add tag filter if specified
        if let Some(tags) = &query.tags {
            if !tags.is_empty() {
                // SQLite doesn't have a direct array contains operator like PostgreSQL
                // So we'll use JSON functions to check if the tag exists in the tags array
                sql.push_str(" AND (
                    SELECT COUNT(*) FROM json_each(tags) 
                    WHERE json_each.value IN (");
                
                let tag_placeholders: Vec<String> = (1..=tags.len())
                    .map(|i| format!("?{}", i + params.len()))
                    .collect();
                    
                sql.push_str(&tag_placeholders.join(", "));
                sql.push_str(")
                ) > 0");
                
                for tag in tags {
                    params.push(Box::new(tag.clone()) as _);
                }
            }
        }
        
        // Add text filter if specified
        if let Some(text_filter) = &query.text_filter {
            let search_term = format!("%{}%", text_filter);
            sql.push_str(" AND (title LIKE ? OR content LIKE ?)");
            params.push(Box::new(search_term.clone()) as _);
            params.push(Box::new(search_term) as _);
        }
        
        // Add ordering
        sql.push_str(" ORDER BY created_at DESC");
        
        // Add limit if specified
        if let Some(limit) = query.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        
        // Execute the query
        let mut query = sql_query(&sql);
        
        // Bind parameters
        for param in params {
            query = query.bind::<Text, _>(param);
        }
        
        let rows = query
            .load::<diesel::sqlite::SqliteRow>(&mut conn)
            .await
            .map_err(|e| Error::other(format!("Failed to query snippets: {}", e)))?;
            
        for row in rows {
            let snippet = self.row_to_snippet(row)?;
            results.push(Box::new(snippet) as Box<dyn crate::memory::MemoryItem + Send + Sync>);
        }
        
        Ok(results)
    }
    
    async fn get_related(
        &self,
        id: &Uuid,
        relation_type: Option<&str>,
    ) -> Result<Vec<Box<dyn crate::memory::MemoryItem + Send + Sync>>> {
        use diesel::prelude::*;
        
        let id_str = id.to_string();
        let mut conn = self.get_conn().await?;
        let mut results = Vec::new();
        
        // First, check if the relations table exists
        let table_exists: bool = sql_query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='snippet_relations'"
        )
        .get_result::<bool>(&mut conn)
        .await
        .unwrap_or(false);
        
        if !table_exists {
            return Ok(Vec::new());
        }
        
        // Build the query based on whether we're filtering by relation type
        let (sql, params) = if let Some(rel_type) = relation_type {
            (
                "
                SELECT s.* FROM snippets s
                JOIN snippet_relations r ON s.uuid = r.to_uuid
                WHERE r.from_uuid = ? AND r.relation_type = ?
                ORDER BY r.created_at DESC
                ".to_string(),
                vec![
                    Box::new(id_str) as Box<dyn diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite> + Send>,
                    Box::new(rel_type.to_string()) as _
                ]
            )
        } else {
            (
                "
                SELECT s.* FROM snippets s
                JOIN snippet_relations r ON s.uuid = r.to_uuid
                WHERE r.from_uuid = ?
                ORDER BY r.created_at DESC
                ".to_string(),
                vec![
                    Box::new(id_str) as Box<dyn diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite> + Send>
                ]
            )
        };
        
        // Execute the query
        let mut query = sql_query(&sql);
        
        // Bind parameters
        for param in params {
            query = query.bind::<Text, _>(param);
        }
        
        let rows = query
            .load::<diesel::sqlite::SqliteRow>(&mut conn)
            .await
            .map_err(|e| Error::other(format!("Failed to get related snippets: {}", e)))?;
            
        for row in rows {
            let snippet = self.row_to_snippet(row)?;
            results.push(Box::new(snippet) as Box<dyn crate::memory::MemoryItem + Send + Sync>);
        }
        
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        database::create_pool,
        models::{Snippet, SnippetWithTags},
    };
    use chrono::Utc;
    use diesel_async::RunQueryDsl;
    use diesel_migrations::{embed_migrations, MigrationHarness};
    use serde_json;
    use std::sync::Arc;
    use uuid::Uuid;

    // This will embed the migrations in the binary
    pub const MIGRATIONS: diesel_migrations::EmbeddedMigrations = embed_migrations!("migrations");

    async fn create_test_backend() -> SqliteBackend {
        // Create a test pool with an in-memory SQLite database
        let database_url = "sqlite::memory:";
        let pool = create_pool(database_url).await.expect("Failed to create test pool");
        
        // Get a connection from the pool to run migrations
        let mut conn = pool.get_async().await.expect("Failed to get connection from pool");
        
        // Run migrations on the same connection that will be used by the tests
        conn.run_pending_migrations(MIGRATIONS)
            .await
            .expect("Failed to run migrations");
        
        // Create the backend with the same pool
        SqliteBackend::new(Arc::new(pool))
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
    
    #[tokio::test]
    async fn test_query() {
        let backend = create_test_backend().await;
        
        // Create some test snippets
        let snippet1 = Snippet {
            uuid: Uuid::new_v4().to_string(),
            title: "Test Snippet 1".to_string(),
            content: "Test content 1".to_string(),
            tags: serde_json::to_string(&vec!["test1".to_string()]).unwrap(),
            embedding: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };
        
        let snippet2 = Snippet {
            uuid: Uuid::new_v4().to_string(),
            title: "Test Snippet 2".to_string(),
            content: "Another test content".to_string(),
            tags: serde_json::to_string(&vec!["test2".to_string()]).unwrap(),
            embedding: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };
        
        // Save the snippets
        backend.save(&snippet1).await.unwrap();
        backend.save(&snippet2).await.unwrap();
        
        // Query with text filter
        let query = crate::models::Query {
            text_filter: Some("Another".to_string()),
            tags: None,
            limit: Some(10),
        };
        
        let results = backend.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        
        let first_result = results[0]
            .as_any()
            .downcast_ref::<SnippetWithTags>()
            .unwrap();
            
        assert_eq!(first_result.title, "Test Snippet 2");
    }
    
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
    
    #[tokio::test]
    async fn test_relations() {
        let backend = create_test_backend().await;
        
        // Create two snippets
        let snippet1 = Snippet {
            uuid: Uuid::new_v4().to_string(),
            title: "Source Snippet".to_string(),
            content: "Source content".to_string(),
            tags: serde_json::to_string(&Vec::<String>::new()).unwrap(),
            embedding: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };
        
        let snippet2 = Snippet {
            uuid: Uuid::new_v4().to_string(),
            title: "Related Snippet".to_string(),
            content: "Related content".to_string(),
            tags: serde_json::to_string(&Vec::<String>::new()).unwrap(),
            embedding: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };
        
        // Save the snippets
        backend.save(&snippet1).await.unwrap();
        backend.save(&snippet2).await.unwrap();
        
        // Add a relation
        let from_id = Uuid::parse_str(&snippet1.uuid).unwrap();
        let to_id = Uuid::parse_str(&snippet2.uuid).unwrap();
        backend.add_relation(&from_id, &to_id, "related").await.unwrap();
        
        // Get related snippets
        let related = backend.get_related(&from_id, Some("related")).await.unwrap();
        assert_eq!(related.len(), 1);
        
        let related_snippet = related[0]
            .as_any()
            .downcast_ref::<SnippetWithTags>()
            .unwrap();
            
        assert_eq!(related_snippet.title, "Related Snippet");
    }
}
