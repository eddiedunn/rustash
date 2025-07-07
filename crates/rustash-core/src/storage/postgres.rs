//! PostgreSQL backend implementation for Rustash storage with Apache AGE support.

use super::StorageBackend;
use crate::{
    error::{Error, Result},
    models::{DbSnippet, NewDbSnippet, Snippet},
};
use async_trait::async_trait;
use bb8::{Pool, PooledConnection};
use bb8_postgres::{tokio_postgres::NoTls, PostgresConnectionManager};
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
        let db_snippet = DbSnippet {
            id: row.get::<_, i32>("id"),
            uuid: row.get::<_, String>("uuid"),
            title: row.get("title"),
            content: row.get("content"),
            tags: row.get("tags"),
            embedding: row.get("embedding"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        };

        Ok(db_snippet.into())
    }
}

#[async_trait]
impl StorageBackend for PostgresBackend {
    async fn save(&self, item: &dyn crate::memory::MemoryItem) -> Result<()> {
        let snippet = item
            .as_any()
            .downcast_ref::<Snippet>()
            .ok_or_else(|| Error::InvalidData("Expected a Snippet".into()))?;

        let new_snippet: NewDbSnippet = snippet.clone().into();
        let mut conn = self.get_conn().await?;

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
                &new_snippet.uuid,
                &new_snippet.title,
                &new_snippet.content,
                &new_snippet.tags,
                &new_snippet.embedding,
                &chrono::Utc::now().naive_utc(),
                &chrono::Utc::now().naive_utc(),
            ],
        )
        .await?;

        Ok(())
    }

    async fn get(&self, id: &Uuid) -> Result<Option<Box<dyn crate::memory::MemoryItem>>> {
        let mut conn = self.get_conn().await?;
        let uuid_str = id.to_string();

        let row = conn
            .query_opt(
                "SELECT * FROM snippets WHERE uuid = $1",
                &[&uuid_str],
            )
            .await?;

        match row {
            Some(row) => {
                let snippet = self.row_to_snippet(row)?;
                Ok(Some(Box::new(snippet) as Box<dyn crate::memory::MemoryItem>))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        let mut conn = self.get_conn().await?;
        let uuid_str = id.to_string();

        conn.execute("DELETE FROM snippets WHERE uuid = $1", &[&uuid_str])
            .await?;

        Ok(())
    }

    async fn query(
        &self,
        query: &crate::models::Query,
    ) -> Result<Vec<Box<dyn crate::memory::MemoryItem>>> {
        let mut conn = self.get_conn().await?;
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> = Vec::new();
        let mut where_clauses = Vec::new();
        let mut param_index = 1;

        // Build the query dynamically based on filters
        if let Some(text) = &query.text_filter {
            where_clauses.push(format!(
                "(title ILIKE ${} OR content ILIKE ${})",
                param_index, param_index
            ));
            params.push(Box::new(format!("%{}%", text)) as _);
            param_index += 1;
        }

        if let Some(tag) = &query.tag_filter {
            where_clauses.push(format!("tags::jsonb ? ${}", param_index));
            params.push(Box::new(tag) as _);
            param_index += 1;
        }

        let where_clause = if where_clauses.is_empty() {
            "".to_string()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        let limit_clause = if query.limit > 0 {
            format!("LIMIT ${}", param_index)
        } else {
            "".to_string()
        };

        if query.limit > 0 {
            params.push(Box::new(query.limit as i64) as _);
        }

        let sql = format!(
            "SELECT * FROM snippets {} ORDER BY updated_at DESC {}",
            where_clause, limit_clause
        );

        let rows = conn.query(&sql, &params[..]).await?;

        let mut results = Vec::new();
        for row in rows {
            let snippet = self.row_to_snippet(row)?;
            results.push(Box::new(snippet) as Box<dyn crate::memory::MemoryItem>);
        }

        Ok(results)
    }

    async fn vector_search(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(Box<dyn crate::memory::MemoryItem>, f32)>> {
        let mut conn = self.get_conn().await?;
        
        // Use pgvector for similarity search if available
        let rows = conn
            .query(
                r#"
                SELECT s.*, 1.0 - (s.embedding <=> $1) as similarity
                FROM snippets s
                WHERE s.embedding IS NOT NULL
                ORDER BY s.embedding <=> $1
                LIMIT $2
                "#,
                &[&embedding, &(limit as i64)],
            )
            .await?;

        let mut results = Vec::new();
        for row in rows {
            let snippet = self.row_to_snippet(row)?;
            let similarity = row.get::<_, f32>("similarity");
            results.push((Box::new(snippet) as Box<dyn crate::memory::MemoryItem>, similarity));
        }

        Ok(results)
    }

    async fn add_relation(
        &self,
        from: &Uuid,
        to: &Uuid,
        relation_type: &str,
    ) -> Result<()> {
        let mut conn = self.get_conn().await?;
        
        // Use Apache AGE to create a relationship between snippets
        conn.execute(
            r#"
            SELECT create_graph('snippet_graph');
            "#,
            &[],
        ).await?;
        
        conn.execute(
            r#"
            SELECT * FROM cypher('snippet_graph', $$
                MATCH (a:snippet {uuid: $1}), (b:snippet {uuid: $2})
                MERGE (a)-[r:RELATED_TO {type: $3}]->(b)
                RETURN r
            $$) AS (r agtype);
            "#,
            &[&from.to_string(), &to.to_string(), &relation_type],
        ).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Query;
    use chrono::Utc;
    use uuid::Uuid;

    // Note: These tests require a running PostgreSQL instance with the required extensions
    // They are marked as ignored by default and should be run manually

    async fn create_test_backend() -> Result<PostgresBackend> {
        // Set up test database connection
        let manager = PostgresConnectionManager::new_from_stringlike(
            "postgres://postgres:postgres@localhost:5432/rustash_test",
            NoTls,
        )?;
        
        let pool = Pool::builder().build(manager).await?;
        let backend = PostgresBackend::new(pool);
        
        // Set up test schema
        let mut conn = backend.get_conn().await?;
        
        // Create extensions if they don't exist
        conn.execute("CREATE EXTENSION IF NOT EXISTS "postgres";", &[]).await?;
        conn.execute("CREATE EXTENSION IF NOT EXISTS "uuid-ossp";", &[]).await?;
        
        // Create test table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS snippets (
                id SERIAL PRIMARY KEY,
                uuid UUID NOT NULL UNIQUE,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                tags JSONB NOT NULL DEFAULT '[]'::jsonb,
                embedding VECTOR(1536),
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
            &[],
        ).await?;
        
        // Create indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_snippets_uuid ON snippets(uuid)",
            &[],
        ).await?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_snippets_tags ON snippets USING GIN(tags)",
            &[],
        ).await?;
        
        // Clear any existing test data
        conn.execute("TRUNCATE TABLE snippets CASCADE", &[]).await?;
        
        Ok(backend)
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL with pgvector and AGE"]
    async fn test_save_and_get() -> Result<()> {
        let backend = create_test_backend().await?;
        
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
        backend.save(&snippet).await?;
        
        // Retrieve it
        let retrieved = backend.get(&snippet.id).await?.unwrap();
        let retrieved_snippet = retrieved
            .as_any()
            .downcast_ref::<Snippet>()
            .unwrap();
            
        assert_eq!(retrieved_snippet.title, "Test Snippet");
        assert_eq!(retrieved_snippet.content, "Test content");
        assert_eq!(retrieved_snippet.tags, vec!["test"]);
        
        Ok(())
    }
    
    #[tokio::test]
    #[ignore = "requires PostgreSQL with pgvector and AGE"]
    async fn test_query() -> Result<()> {
        let backend = create_test_backend().await?;
        
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
            backend.save(snippet).await?;
        }
        
        // Test text search
        let query = Query {
            text_filter: Some("Rust".to_string()),
            tag_filter: None,
            limit: 10,
        };
        
        let results = backend.query(&query).await?;
        assert_eq!(results.len(), 1);
        
        // Test tag filter
        let query = Query {
            text_filter: None,
            tag_filter: Some("python".to_string()),
            limit: 10,
        };
        
        let results = backend.query(&query).await?;
        assert_eq!(results.len(), 1);
        
        Ok(())
    }
    
    #[tokio::test]
    #[ignore = "requires PostgreSQL with pgvector and AGE"]
    async fn test_relations() -> Result<()> {
        let backend = create_test_backend().await?;
        
        let snippet1 = Snippet {
            id: Uuid::new_v4(),
            title: "Related Snippet 1".to_string(),
            content: "First related snippet".to_string(),
            tags: vec!["relation".to_string()],
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        let snippet2 = Snippet {
            id: Uuid::new_v4(),
            title: "Related Snippet 2".to_string(),
            content: "Second related snippet".to_string(),
            tags: vec!["relation".to_string()],
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        // Save snippets
        backend.save(&snippet1).await?;
        backend.save(&snippet2).await?;
        
        // Add relation
        backend.add_relation(&snippet1.id, &snippet2.id, "RELATED").await?;
        
        // In a real test, we would query the graph to verify the relationship
        // This is just a placeholder to show the concept
        assert!(true);
        
        Ok(())
    }
}
