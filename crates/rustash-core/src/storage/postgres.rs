//! PostgreSQL backend implementation for Rustash storage.

use super::StorageBackend;
use crate::{
    database::postgres_pool::PgPool,
    error::{Error, Result},
    models::{Query, Snippet, SnippetWithTags},
};
use chrono::{DateTime, Utc};
use diesel::{
    pg::Pg,
    query_builder::QueryFragment,
    sql_types::{Text, Timestamptz, Uuid as SqlUuid},
    Connection as _, ExpressionMethods, QueryDsl, RunQueryDsl,
};
use diesel_async::{
    pg::PgRow,
    pooled_connection::bb8::PooledConnection,
    AsyncConnection, AsyncPgConnection, RunQueryDsl as _,
};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

/// A PostgreSQL-backed storage implementation.
#[derive(Debug, Clone)]
pub struct PostgresBackend {
    pool: std::sync::Arc<PgPool>,
}

impl PostgresBackend {
    /// Create a new PostgreSQL backend with the given connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool: std::sync::Arc::new(pool) }
    }

    /// Get a connection from the pool.
    async fn get_conn(&self) -> Result<diesel_async::pooled_connection::bb8::PooledConnection<'_, diesel_async::pooled_connection::AsyncDieselConnectionManager<diesel_async::AsyncPgConnection>>> {
    self.pool.get().await.map_err(|e| Error::Pool(e.to_string()))
}


}

#[async_trait]
impl StorageBackend for PostgresBackend {
    async fn save(&self, item: &(dyn crate::memory::MemoryItem + Send + Sync)) -> Result<()> {
        let snippet = item
            .as_any()
            .downcast_ref::<SnippetWithTags>()
            .ok_or_else(|| Error::other("Expected a SnippetWithTags"))?;

        // Validate the UUID format
        Uuid::parse_str(&snippet.uuid)
            .map_err(|e| Error::other(format!("Invalid UUID format: {}", e)))?;

        let now = chrono::Utc::now();
        let embedding_bytes = snippet.embedding.as_ref().map(|b| b.to_vec());
        let mut conn = self.get_conn().await?;
        let query = r#"
            INSERT INTO snippets (uuid, title, content, tags, embedding, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (uuid) DO UPDATE
            SET title = EXCLUDED.title,
                content = EXCLUDED.content,
                tags = EXCLUDED.tags,
                embedding = EXCLUDED.embedding,
                updated_at = EXCLUDED.updated_at
        "#;
        use diesel::sql_types::{Text, Timestamptz};
        use diesel::sql_types::Jsonb;
        use diesel::sql_types::Nullable;
        diesel::sql_query(query)
            .bind::<Text, _>(&snippet.uuid)
            .bind::<Text, _>(&snippet.title)
            .bind::<Text, _>(&snippet.content)
            .bind::<Jsonb, _>(serde_json::to_value(&snippet.tags)? )
            .bind::<Nullable<diesel::sql_types::Binary>, _>(embedding_bytes)
            .bind::<Timestamptz, _>(now)
            .bind::<Timestamptz, _>(now)
            .execute(&mut *conn)
            .await
            .map_err(|e| Error::other(format!("Failed to save snippet: {}", e)))?;
        Ok(())
    }
        let now = chrono::Utc::now().naive_utc();

        let mut conn = self.get_conn().await?;

        // Use parameterized query to prevent SQL injection
        let query = r#"
            INSERT INTO snippets (uuid, title, content, tags, embedding, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (uuid) DO UPDATE
            SET title = EXCLUDED.title,
                content = EXCLUDED.content,
                tags = EXCLUDED.tags,
                embedding = EXCLUDED.embedding,
                updated_at = EXCLUDED.updated_at
        "#;

        sql_query(query)
            .bind::<Text, _>(&snippet.uuid)
            .bind::<Text, _>(&snippet.title)
            .bind::<Text, _>(&snippet.content)
            .bind::<Text, _>(&snippet.tags)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Binary>, _>(
                snippet.embedding.as_ref(),
            )
            .bind::<diesel::sql_types::Timestamp, _>(now)
            .bind::<diesel::sql_types::Timestamp, _>(now)
            .execute(&mut conn)
            .await
            .map_err(|e| Error::other(format!("Failed to save snippet: {}", e)))?;

        Ok(())
    }

    async fn get(
        &self,
        id: &Uuid,
    ) -> Result<Option<Box<dyn crate::memory::MemoryItem + Send + Sync>>> {
        use crate::schema::snippets::dsl::*;
        let id_str = id.to_string();
        let mut conn = self.get_conn().await?;
        let result: Option<DbSnippet> = snippets
            .filter(uuid.eq(&id_str))
            .first::<DbSnippet>(&mut *conn)
            .await
            .optional()?;
        match result {
            Some(db_snippet) => {
                let snippet_with_tags: SnippetWithTags = db_snippet.into();
                Ok(Some(Box::new(snippet_with_tags)))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        use diesel::prelude::*;

        let id_str = id.to_string();
        let mut conn = self.get_conn().await?;

        let query = "DELETE FROM snippets WHERE uuid = $1";

        sql_query(query)
            .bind::<Text, _>(&id_str)
            .execute(&mut conn)
            .await
            .map_err(|e| Error::other(format!("Failed to delete snippet: {}", e)))?;

        Ok(())
    }

    async fn query(
        &self,
        query: &crate::models::Query,
    ) -> Result<Vec<Box<dyn crate::memory::MemoryItem + Send + Sync>>> {
        use crate::schema::snippets;
        use diesel::prelude::*;

        let mut conn = self.get_conn().await?;
        let mut query_builder = snippets::table.into_boxed();

        if let Some(text_filter) = &query.text_filter {
            let search_term = format!("%{}%", text_filter);
            query_builder = query_builder.filter(
                snippets::title
                    .ilike(&search_term)
                    .or(snippets::content.ilike(&search_term)),
            );
        }
        if let Some(tags) = &query.tags {
            if !tags.is_empty() {
                use diesel::dsl::sql;
                let tags_json = serde_json::to_value(tags)?;
                query_builder = query_builder.filter(sql::<diesel::sql_types::Bool>(&format!("tags @> '{}'", tags_json)));
            }
        }
        if let Some(limit) = query.limit {
            query_builder = query_builder.limit(limit as i64);
        }
        let db_snippets = query_builder.load::<DbSnippet>(&mut *conn).await?;
        let results: Vec<Box<dyn MemoryItem + Send + Sync>> = db_snippets
            .into_iter()
            .map(|s| Box::new(SnippetWithTags::from(s)) as Box<dyn MemoryItem + Send + Sync>)
            .collect();
        Ok(results)
    }

    async fn vector_search(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(Box<dyn crate::memory::MemoryItem + Send + Sync>, f32)>> {
        use diesel::sql_types::{BigInt, Float};
        let query_vector = Vector::from(embedding.to_vec());
        #[derive(QueryableByName)]
        struct SnippetWithDistance {
            #[diesel(embed)]
            snippet: DbSnippet,
            #[diesel(sql_type = "Float")]
            distance: f32,
        }
        let query = diesel::sql_query(
            "SELECT *, embedding <-> $1 AS distance FROM snippets
             WHERE embedding IS NOT NULL
             ORDER BY distance ASC
             LIMIT $2",
        )
        .bind::<pgvector::sql_types::Vector, _>(&query_vector)
        .bind::<BigInt, _>(limit as i64);
        let mut conn = self.get_conn().await?;
        let rows: Vec<SnippetWithDistance> = query.get_results(&mut *conn).await?;
        let results = rows
            .into_iter()
            .map(|row| {
                let with_tags: SnippetWithTags = row.snippet.into();
                (
                    Box::new(with_tags) as Box<dyn crate::memory::MemoryItem + Send + Sync>,
                    row.distance,
                )
            })
            .collect();
        Ok(results)
    }

    async fn add_relation(&self, _from: &Uuid, _to: &Uuid, _relation_type: &str) -> Result<()> {
        // TODO: Reimplement with proper async Diesel support for graph queries
        // For now, this is a no-op as we need to properly implement the graph functionality
        // with the new connection pool setup
        Ok(())
        
        /*
        let mut conn = self.get_conn().await?;

        // Ensure the graph nodes exist first. This is idempotent.
        let upsert_from_query = format!("MERGE (a:Snippet {{uuid: '{}'}})", from);
        let upsert_to_query = format!("MERGE (b:Snippet {{uuid: '{}'}})", to);
        
        // Execute the Cypher queries directly using the connection
        diesel::sql_query("SELECT * from age.cypher('rustash_graph', $1) as (v agtype);")
            .bind::<diesel::sql_types::Text, _>(&upsert_from_query)
            .execute(&mut conn)
            .await?;
            
        diesel::sql_query("SELECT * from age.cypher('rustash_graph', $1) as (v agtype);")
            .bind::<diesel::sql_types::Text, _>(&upsert_to_query)
            .execute(&mut conn)
            .await?;

        // Now create the relationship
        let cypher_query = format!(
            "MATCH (a:Snippet {{uuid: '{}'}}), (b:Snippet {{uuid: '{}'}})
             MERGE (a)-[:{}]->(b)",
            from,
            to,
            relation_type.to_uppercase()
        );
            
        diesel::sql_query("SELECT * from age.cypher('rustash_graph', $1) as (v agtype);")
            .bind::<diesel::sql_types::Text, _>(&cypher_query)
            .execute(&mut conn)
            .await?;
        
        Ok(())
        */
    }

    async fn get_related(
        &self,
        id: &Uuid,
        _relation_type: Option<&str>,
    ) -> Result<Vec<Box<dyn crate::memory::MemoryItem + Send + Sync>>> {
        // For now, return an empty vector as we need to reimplement the graph query
        // with proper async Diesel support
        Ok(vec![])
        
        /*
        // TODO: Reimplement with proper async Diesel support for graph queries
        let mut conn = self.get_conn().await?;
        
        // This is a simplified implementation that just returns related snippets
        // by looking up relations in the database
        let related: Vec<Snippet> = crate::schema::snippets::table
            .filter(crate::schema::snippets::uuid.ne(id.to_string()))
            .limit(10) // Limit to 10 related items for now
            .load(&mut conn)
            .await?;
            
        let results = related
            .into_iter()
            .map(|s| Box::new(s) as Box<dyn crate::memory::MemoryItem + Send + Sync>)
            .collect();
            
        Ok(results)
        */
    }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        database::create_connection_pool,
        models::{Snippet, SnippetWithTags},
    };
    use chrono::Utc;
    use diesel_migrations::{embed_migrations, MigrationHarness};
    use std::sync::Arc;
    use uuid::Uuid;

    // This will embed the migrations in the binary
    pub const MIGRATIONS: diesel_migrations::EmbeddedMigrations = embed_migrations!("migrations");

    async fn create_test_backend() -> Result<PostgresBackend> {
        // Set up test database connection
        let database_url = "postgres://postgres:postgres@localhost:5432/rustash_test";
        let pool = create_connection_pool(database_url).await?;

        // Get a connection from the pool to run migrations
        let mut conn = pool.get().await?;

        // Run migrations on the same connection that will be used by the tests
        conn.run_pending_migrations(MIGRATIONS)
            .await
            .expect("Failed to run migrations");

        // Create the backend with the same pool
        Ok(PostgresBackend::new(Arc::new(pool)))
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL with pgvector and AGE"]
    async fn test_save_and_get() {
        let backend = create_test_backend().await.unwrap();

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
    #[ignore = "requires PostgreSQL with pgvector and AGE"]
    async fn test_save_and_update() {
        let backend = create_test_backend().await.unwrap();

        let snippet_id = Uuid::new_v4();
        let mut snippet = Snippet {
            uuid: snippet_id.to_string(),
            title: "Initial Title".to_string(),
            content: "Initial content".to_string(),
            tags: serde_json::to_string(&vec!["initial".to_string()]).unwrap(),
            embedding: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };

        backend.save(&snippet).await.unwrap();

        snippet.title = "Updated Title".to_string();
        snippet.content = "Updated content".to_string();

        backend.save(&snippet).await.unwrap();

        let retrieved = backend.get(&snippet_id).await.unwrap().unwrap();
        let retrieved_snippet = retrieved
            .as_any()
            .downcast_ref::<SnippetWithTags>()
            .unwrap();

        assert_eq!(retrieved_snippet.title, "Updated Title");
        assert_eq!(retrieved_snippet.content, "Updated content");
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL with pgvector and AGE"]
    async fn test_query() {
        let backend = create_test_backend().await.unwrap();

        // Create some test snippets
        let snippet1 = Snippet {
            uuid: Uuid::new_v4().to_string(),
            title: "Python List".to_string(),
            content: "my_list = [1, 2, 3]".to_string(),
            tags: serde_json::to_string(&vec!["python".to_string()]).unwrap(),
            embedding: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };

        let snippet2 = Snippet {
            uuid: Uuid::new_v4().to_string(),
            title: "Rust Vector".to_string(),
            content: "let vec = vec![1, 2, 3];".to_string(),
            tags: serde_json::to_string(&vec!["rust".to_string()]).unwrap(),
            embedding: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };

        // Save the snippets
        backend.save(&snippet1).await.unwrap();
        backend.save(&snippet2).await.unwrap();

        // Query with text filter
        let query = crate::models::Query {
            text_filter: Some("vector".to_string()),
            tags: Some(vec!["rust".to_string()]),
            limit: Some(10),
        };

        let results = backend.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);

        let result = results[0]
            .as_any()
            .downcast_ref::<SnippetWithTags>()
            .unwrap();

        assert_eq!(result.title, "Rust Vector");
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL with pgvector and AGE"]
    async fn test_vector_search() {
        let backend = create_test_backend().await.unwrap();

        // Create a test snippet with an embedding
        let snippet = Snippet {
            uuid: Uuid::new_v4().to_string(),
            title: "Vector Test".to_string(),
            content: "This is a test for vector search".to_string(),
            tags: serde_json::to_string(&vec!["test".to_string()]).unwrap(),
            embedding: Some(vec![0.1, 0.2, 0.3]),
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };

        // Save the snippet
        backend.save(&snippet).await.unwrap();

        // Search with a similar vector
        let query_embedding = vec![0.11, 0.19, 0.31]; // Similar to the saved embedding
        let results = backend.vector_search(&query_embedding, 5).await.unwrap();

        // Should find our snippet
        assert!(!results.is_empty());
        let (result, _score) = &results[0];
        let result_snippet = result.as_any().downcast_ref::<SnippetWithTags>().unwrap();

        assert_eq!(result_snippet.title, "Vector Test");
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL with pgvector and AGE"]
    async fn test_relations() {
        let backend = create_test_backend().await.unwrap();

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
        backend
            .add_relation(&from_id, &to_id, "related")
            .await
            .unwrap();

        // Get related snippets
        let related = backend
            .get_related(&from_id, Some("related"))
            .await
            .unwrap();
        assert_eq!(related.len(), 1);

        let related_snippet = related[0]
            .as_any()
            .downcast_ref::<SnippetWithTags>()
            .unwrap();

        assert_eq!(related_snippet.title, "Related Snippet");
    }
}
