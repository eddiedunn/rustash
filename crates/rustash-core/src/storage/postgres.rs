//! PostgreSQL backend implementation for Rustash storage with Apache AGE support.

use super::StorageBackend;
use crate::{
    error::{Error, Result},
    models::{DbSnippet, NewDbSnippet, Query, Snippet},
};
use async_trait::async_trait;
use bb8::{Pool, PooledConnection};
use bb8_postgres::{tokio_postgres::NoTls, PostgresConnectionManager};
use chrono::{DateTime, NaiveDateTime, Utc};
use std::sync::Arc;
use tokio_postgres::Row;
use uuid::Uuid;

/// A PostgreSQL-backed storage implementation with Apache AGE support.
#[derive(Debug, Clone)]
pub struct PostgresBackend {
    pool: Arc<Pool<PostgresConnectionManager<NoTls>>>,
}

impl PostgresBackend {
    /// Create a new PostgreSQL backend with the given connection pool.
    pub fn new(pool: Pool<PostgresConnectionManager<NoTls>>) -> Self {
        Self { pool: Arc::new(pool) }
    }

    /// Get a connection from the pool.
    async fn get_conn(
        &self,
    ) -> Result<PooledConnection<'_, PostgresConnectionManager<NoTls>>> {
        self.pool.get_owned().await.map_err(Error::from)
    }

    /// Convert a database row to a Snippet
    fn row_to_snippet(&self, row: Row) -> Result<Snippet> {
        let uuid_str: String = row.get("uuid");
        let _ = Uuid::parse_str(&uuid_str).map_err(Error::from)?;
        let tags_json: String = row.get("tags");
        let _: Vec<String> = serde_json::from_str(&tags_json).map_err(Error::from)?;
        
        // Use NaiveDateTime directly since that's what's stored in the database
        let created_at: NaiveDateTime = row.get("created_at");
        let updated_at: NaiveDateTime = row.get("updated_at");
        
        let snippet = Snippet {
            uuid: uuid_str,
            title: row.get("title"),
            content: row.get("content"),
            tags: tags_json,
            embedding: row.get("embedding"),
            created_at,
            updated_at,
        };

        Ok(snippet)
    }
}

#[async_trait]
impl StorageBackend for PostgresBackend {
    async fn save(&self, item: &(dyn crate::memory::MemoryItem + Send + Sync)) -> Result<()> {
        let snippet = item
            .as_any()
            .downcast_ref::<Snippet>()
            .ok_or_else(|| Error::other("Expected a Snippet"))?;

        let uuid = Uuid::parse_str(&snippet.uuid).map_err(|e| Error::other(format!("Invalid UUID format: {}", e)))?;
        let tags: Vec<String> = serde_json::from_str(&snippet.tags).unwrap_or_default();
        let tags_json = serde_json::to_string(&tags).map_err(|e| Error::other(format!("Invalid tags format: {}", e)))?;
        let now = chrono::Utc::now();
        
        let conn = self.get_conn().await?;

        conn.execute(
            r#"
            INSERT INTO snippets (uuid, title, content, tags, embedding, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (uuid) DO UPDATE
            SET title = EXCLUDED.title,
                content = EXCLUDED.content,
                tags = EXCLUDED.tags,
                embedding = EXCLUDED.embedding,
                updated_at = EXCLUDED.updated_at
            "#,
            &[
                &snippet.uuid,
                &snippet.title,
                &snippet.content,
                &tags_json,
                &snippet.embedding,
                &now.naive_utc(),
                &now.naive_utc(),
            ],
        )
        .await
        .map_err(|e| Error::other(e.to_string()))?;

        Ok(())
    }

    async fn get(&self, id: &Uuid) -> Result<Option<Box<dyn crate::memory::MemoryItem + Send + Sync>>> {
        let conn = self.get_conn().await?;
        let id_str = id.to_string();
        
        let row = conn
            .query_opt(
                "SELECT * FROM snippets WHERE uuid = $1",
                &[&id_str],
            )
            .await
            .map_err(|e| Error::other(format!("Database query failed: {}", e)))?;

        match row {
            Some(row) => {
                let snippet = self.row_to_snippet(row)?;
                Ok(Some(Box::new(snippet) as Box<dyn crate::memory::MemoryItem>))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        let conn = self.get_conn().await?;
        let uuid_str = id.to_string();

        conn.execute("DELETE FROM snippets WHERE uuid = $1", &[&uuid_str])
            .await
            .map_err(|e| Error::other(e.to_string()))?;

        Ok(())
    }

    async fn query(
        &self,
        query: &crate::models::Query,
    ) -> Result<Vec<Box<dyn crate::memory::MemoryItem + Send + Sync>>> {
        use tokio_postgres::types::ToSql;
        
        let conn = self.get_conn().await?;
        let mut results = Vec::new();
        let mut sql = "SELECT * FROM snippets".to_string();
        let mut where_clauses = Vec::new();
        let mut params: Vec<Box<dyn ToSql + Send + Sync>> = Vec::new();
        let mut param_index = 1;

        // Handle text filter
        if let Some(text) = &query.text_filter {
            where_clauses.push(format!("content ILIKE ${}", param_index));
            let search_term = format!("%{}%", text);
            params.push(Box::new(search_term) as Box<dyn ToSql + Send + Sync>);
            param_index += 1;
        }

        // Handle tag filter - using the same text_filter field as per Query struct
        if let Some(tag) = &query.text_filter {
            where_clauses.push(format!("${} = ANY(string_to_array(tags, ','))", param_index));
            params.push(Box::new(tag.clone()) as Box<dyn ToSql + Send + Sync>);
            param_index += 1;
        }

        // Add WHERE clause if we have any conditions
        if !where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_clauses.join(" AND "));
        }

        // Add LIMIT if specified (query.limit is a usize, not an Option)
        if query.limit > 0 {
            sql.push_str(&format!(" LIMIT ${}", param_index));
            params.push(Box::new(query.limit as i64) as Box<dyn ToSql + Send + Sync>);
        }

        // Execute the query with parameters
        let param_refs: Vec<&(dyn ToSql + Sync)> = params.iter().map(|p| &**p as &(dyn ToSql + Sync)).collect();
        let rows = conn.query(&sql, &param_refs[..]).await?;

        // Process results
        for row in rows {
            let snippet = self.row_to_snippet(row)?;
            results.push(Box::new(snippet) as Box<dyn crate::memory::MemoryItem + Send + Sync>);
        }

        Ok(results)
    }

    async fn vector_search(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(Box<dyn crate::memory::MemoryItem + Send + Sync + 'static>, f32)>> {
        // This is a placeholder implementation that doesn't actually do vector search
        // In a real implementation, you would use pgvector or similar for vector similarity search
        let mut results = Vec::new();
        
        let conn = self.get_conn().await?;
        let rows = conn
            .query(
                "SELECT * FROM snippets ORDER BY random() LIMIT $1",
                &[&(limit as i32)],
            )
            .await
            .map_err(Error::from)?;
            
        for row in rows {
            let snippet = self.row_to_snippet(row)?;
            // Assign a dummy similarity score
            results.push((
                Box::new(snippet) as Box<dyn crate::memory::MemoryItem + Send + Sync + 'static>,
                1.0, // Dummy similarity score
            ));
        }
        
        Ok(results)
    }

    async fn add_relation(
        &self,
        from: &Uuid,
        to: &Uuid,
        relation_type: &str,
    ) -> Result<()> {
        let conn = self.get_conn().await?;
        let from_str = from.to_string();
        let to_str = to.to_string();
        
        // Check if the relation already exists
        let exists = conn
            .query_opt(
                "SELECT 1 FROM relations WHERE from_id = $1 AND to_id = $2 AND relation_type = $3",
                &[&from_str, &to_str, &relation_type],
            )
            .await?;
            
        if exists.is_none() {
            conn.execute(
                "INSERT INTO relations (from_id, to_id, relation_type) VALUES ($1, $2, $3)",
                &[&from_str, &to_str, &relation_type],
            )
            .await?;
        }
        
        Ok(())
    }

    async fn get_related(
        &self,
        id: &Uuid,
        relation_type: Option<&str>,
    ) -> Result<Vec<Box<dyn crate::memory::MemoryItem + Send + Sync>>> {
        use tokio_postgres::types::ToSql;
        
        let conn = self.get_conn().await?;
        let id_str = id.to_string();
        
        // Create a vector to hold parameters
        let mut params: Vec<Box<dyn ToSql + Send + Sync>> = Vec::new();
        params.push(Box::new(id_str) as Box<dyn ToSql + Send + Sync>);
        
        // Build the query based on whether we have a relation type
        let (sql, param_refs) = if let Some(rel_type) = relation_type {
            params.push(Box::new(rel_type.to_string()) as Box<dyn ToSql + Send + Sync>);
            
            let sql = "
                SELECT s.* FROM snippets s 
                JOIN snippet_relations sr ON s.uuid = sr.to_snippet_uuid 
                WHERE sr.from_snippet_uuid = $1 AND sr.relation_type = $2
            ".to_string();
            
            let param_refs: Vec<&(dyn ToSql + Sync)> = params.iter().map(|p| &**p as &(dyn ToSql + Sync)).collect();
            (sql, param_refs)
        } else {
            let sql = "
                SELECT s.* FROM snippets s 
                JOIN snippet_relations sr ON s.uuid = sr.to_snippet_uuid 
                WHERE sr.from_snippet_uuid = $1
            ".to_string();
            
            let param_refs: Vec<&(dyn ToSql + Sync)> = params.iter().map(|p| &**p as &(dyn ToSql + Sync)).collect();
            (sql, param_refs)
        };

        // Execute the query
        let rows = conn.query(&sql, &param_refs[..]).await?;

        // Process results
        let mut results = Vec::new();
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
    use crate::models::Snippet;
    use chrono::{DateTime, NaiveDateTime, Utc};
    use std::env;
    use uuid::Uuid;
    
    // Helper function to get current time as NaiveDateTime for testing
    fn now_naive() -> NaiveDateTime {
        Utc::now().naive_utc()
    }

    // Note: These tests require a running PostgreSQL instance with the required extensions
    // They are marked as ignored by default and should be run manually

    async fn create_test_backend() -> Result<PostgresBackend> {
        use tokio_postgres::types::ToSql;
        
        // Set up test database connection
        let manager = PostgresConnectionManager::new_from_stringlike(
            "postgres://postgres:postgres@localhost:5432/rustash_test",
            NoTls,
        )?;
        
            let pool = Pool::builder().build(manager).await?;
        
        // Create a new connection for schema setup
        let conn = pool.get().await?;
        
        // Create extensions if they don't exist
        conn.execute("CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\"", &[]).await?;
        
        // Clone the pool before creating the backend
        let pool_clone = pool.clone();
        let backend = PostgresBackend::new(pool_clone);
        
        // Create test table if it doesn't exist
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS snippets (
                id SERIAL PRIMARY KEY,
                uuid UUID NOT NULL UNIQUE,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                tags TEXT NOT NULL DEFAULT '[]',
                embedding FLOAT4[],
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
            &[] as &[&(dyn ToSql + Sync)],
        ).await?;
        
        // Create indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_snippets_uuid ON snippets(uuid)",
            &[] as &[&(dyn ToSql + Sync)],
        ).await?;
        
        // Create snippet_relations table for testing
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS snippet_relations (
                id SERIAL PRIMARY KEY,
                from_snippet_uuid UUID NOT NULL,
                to_snippet_uuid UUID NOT NULL,
                relation_type TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                UNIQUE(from_snippet_uuid, to_snippet_uuid, relation_type)
            )
            "#,
            &[] as &[&(dyn ToSql + Sync)],
        ).await?;
        
        // Clear any existing test data
        conn.execute("TRUNCATE TABLE snippets, snippet_relations CASCADE", 
            &[] as &[&(dyn ToSql + Sync)],
        ).await?;
        
        Ok(backend)
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL with pgvector and AGE"]
    async fn test_save_and_get() -> Result<()> {
        let backend = create_test_backend().await?;
        let uuid = Uuid::new_v4();
        let now = now_naive();
        
        let snippet = Snippet {
            uuid: uuid.to_string(),
            title: "Test Snippet".to_string(),
            content: "Test content".to_string(),
            tags: serde_json::to_string(&vec!["test".to_string()]).unwrap(),
            embedding: None,
            created_at: now,
            updated_at: now,
        };
        
        // Save the snippet
        backend.save(&snippet).await?;
        
        // Retrieve it
        let retrieved = backend.get(&uuid).await?;
        assert!(retrieved.is_some(), "Snippet not found after save");
        
        let binding = retrieved.unwrap();
        let retrieved_snippet = binding
            .as_any()
            .downcast_ref::<Snippet>()
            .expect("Failed to downcast to Snippet");
            
        assert_eq!(retrieved_snippet.title, "Test Snippet");
        assert_eq!(retrieved_snippet.content, "Test content");
        let tags: Vec<String> = serde_json::from_str(&retrieved_snippet.tags)
            .expect("Failed to deserialize tags");
        assert_eq!(tags, vec!["test"]);
        
        Ok(())
    }
    
    #[tokio::test]
    #[ignore = "requires PostgreSQL with pgvector and AGE"]
    async fn test_query() -> Result<()> {
        let backend = create_test_backend().await?;
        
        // Add test snippets
        let snippets = vec![
            Snippet {
                uuid: Uuid::new_v4().to_string(),
                title: "Python List".to_string(),
                content: "my_list = [1, 2, 3]".to_string(),
                tags: serde_json::to_string(&vec!["python".to_string(), "data-structures".to_string()])
                    .expect("Failed to serialize tags"),
                embedding: None,
                created_at: now_naive(),
                updated_at: now_naive(),
            },
            Snippet {
                uuid: Uuid::new_v4().to_string(),
                title: "Rust Vector".to_string(),
                content: "let vec = vec![1, 2, 3];".to_string(),
                tags: serde_json::to_string(&vec!["rust".to_string(), "data-structures".to_string()]).unwrap(),
                embedding: None,
                created_at: now_naive(),
                updated_at: now_naive(),
            },
        ];

        for snippet in &snippets {
            backend.save(snippet).await?;
        }

        // Test search by tag
        let query = crate::models::Query {
            tags: Some(vec!["python".to_string()]),
            ..Default::default()
        };
        let results = backend.query(&query).await?;
        assert_eq!(results.len(), 1);
        let result_snippet = results[0]
            .as_any()
            .downcast_ref::<Snippet>()
            .unwrap();
        assert_eq!(result_snippet.title, "Python List");

        // Test search by content
        let query = crate::models::Query {
            content: Some("vector".to_string()),
            ..Default::default()
        };
        let results = backend.query(&query).await?;
        assert_eq!(results.len(), 1);
        let result_snippet = results[0]
            .as_any()
            .downcast_ref::<Snippet>()
            .unwrap();
        assert_eq!(result_snippet.title, "Rust Vector");

        Ok(())
    }
    
    #[tokio::test]
    #[ignore = "requires PostgreSQL with pgvector and AGE"]
    async fn test_relations() -> Result<()> {
        let backend = create_test_backend().await?;
        let now = now_naive();
        
        // Create related snippets
        let snippet1 = Snippet {
            uuid: Uuid::new_v4().to_string(),
            title: "Parent Snippet".to_string(),
            content: "Parent content".to_string(),
            tags: serde_json::to_string(&vec!["relation".to_string()]).unwrap(),
            embedding: None,
            created_at: now,
            updated_at: now,
        };
        
        let snippet2 = Snippet {
            uuid: Uuid::new_v4().to_string(),
            title: "Child Snippet".to_string(),
            content: "Child content".to_string(),
            tags: serde_json::to_string(&vec!["relation".to_string()]).unwrap(),
            embedding: None,
            created_at: now,
            updated_at: now,
        };
        
        // Save snippets
        backend.save(&snippet1).await?;
        backend.save(&snippet2).await?;
        
        // Add relation
        let uuid1 = Uuid::parse_str(&snippet1.uuid).unwrap();
        let uuid2 = Uuid::parse_str(&snippet2.uuid).unwrap();
        backend.add_relation(&uuid1, &uuid2, "RELATED").await?;
        
        // Test getting related snippets
        let related = backend.get_related(&uuid1, Some("RELATED")).await?;
        assert_eq!(related.len(), 1);
        let related_snippet = related[0]
            .as_any()
            .downcast_ref::<Snippet>()
            .unwrap();
        assert_eq!(related_snippet.title, "Child Snippet");
        
        Ok(())
    }
}
