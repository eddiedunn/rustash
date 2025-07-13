//! PostgreSQL backend implementation for Rustash storage.

use super::StorageBackend;
use crate::{
    database::PostgresPool,
    error::{Error, Result},
    models::{Snippet, SnippetWithTags},
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use diesel::{prelude::*, sql_query, sql_types::Text};
use diesel_async::{
    async_connection_wrapper::AsyncConnectionWrapper, pg::PgRow, AsyncPgConnection, RunQueryDsl,
};
use std::sync::Arc;
use uuid::Uuid;

/// A PostgreSQL-backed storage implementation.
#[derive(Debug, Clone)]
pub struct PostgresBackend {
    pool: Arc<PostgresPool>,
}

impl PostgresBackend {
    /// Create a new PostgreSQL backend with the given connection pool.
    pub fn new(pool: PostgresPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    /// Get a connection from the pool.
    async fn get_conn(&self) -> Result<AsyncConnectionWrapper<AsyncPgConnection>> {
        Ok(AsyncConnectionWrapper::from(self.pool.get().await?))
    }

    /// Convert a database row to a Snippet
    fn row_to_snippet(&self, row: diesel::pg::PgRow) -> Result<Snippet> {
        use diesel::row::NamedRow;

        let uuid: String = row.get("uuid").map_err(Error::from)?;
        let title: String = row.get("title").map_err(Error::from)?;
        let content: String = row.get("content").map_err(Error::from)?;
        let tags_json: String = row.get("tags").map_err(Error::from)?;
        let embedding: Option<Vec<u8>> = row.get("embedding").map_err(Error::from)?;
        let created_at: NaiveDateTime = row.get("created_at").map_err(Error::from)?;
        let updated_at: NaiveDateTime = row.get("updated_at").map_err(Error::from)?;

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
impl StorageBackend for PostgresBackend {
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
        use diesel::prelude::*;

        let id_str = id.to_string();
        let mut conn = self.get_conn().await?;

        let query = "SELECT * FROM snippets WHERE uuid = $1";

        let result = sql_query(query)
            .bind::<Text, _>(&id_str)
            .get_result::<diesel::pg::PgRow>(&mut conn)
            .await;

        match result {
            Ok(row) => {
                let snippet = self.row_to_snippet(row)?;
                Ok(Some(Box::new(snippet)))
            }
            Err(diesel::result::Error::NotFound) => Ok(None),
            Err(e) => Err(Error::other(format!("Failed to get snippet: {}", e))),
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
        use diesel::prelude::*;

        let mut conn = self.get_conn().await?;
        let mut results = Vec::new();

        // Start building the query
        let mut sql = "SELECT * FROM snippets WHERE 1=1".to_string();
        let mut params: Vec<Box<dyn diesel::query_builder::QueryFragment<Pg> + Send>> = Vec::new();

        // Add tag filter if specified
        if let Some(tags) = &query.tags {
            if !tags.is_empty() {
                sql.push_str(" AND tags @> $1");
                params.push(Box::new(tags.clone()) as _);
            }
        }

        // Add text filter if specified
        if let Some(text_filter) = &query.text_filter {
            let search_term = format!("%{}%", text_filter);
            sql.push_str(" AND (title ILIKE $2 OR content ILIKE $2)");
            params.push(Box::new(search_term) as _);
        }

        // Add ordering
        sql.push_str(" ORDER BY created_at DESC");

        // Add limit if specified
        if let Some(limit) = query.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        // Execute the query
        let rows = sql_query(&sql)
            .load::<diesel::pg::PgRow>(&mut conn)
            .await
            .map_err(|e| Error::other(format!("Failed to query snippets: {}", e)))?;

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
    ) -> Result<Vec<(Box<dyn crate::memory::MemoryItem + Send + Sync>, f32)>> {
        use diesel::sql_types::{BigInt, Bytea, Float};
        use pgvector::Vector;

        let query_vector = Vector::from(embedding.to_vec());

        // This query calculates the L2 distance between the stored embedding
        // and the query vector, ordering by that distance to find the nearest neighbors.
        let query = diesel::sql_query(
            "SELECT *, embedding <-> $1 AS distance FROM snippets
             WHERE embedding IS NOT NULL
             ORDER BY distance ASC
             LIMIT $2",
        )
        .bind::<pgvector::sql_types::Vector, _>(&query_vector)
        .bind::<BigInt, _>(limit as i64);

        #[derive(QueryableByName)]
        struct SnippetWithDistance {
            #[diesel(embed)]
            snippet: Snippet,
            #[diesel(sql_type = "Float")]
            distance: f32,
        }

        let mut conn = self.get_conn().await?;
        let rows: Vec<SnippetWithDistance> = query.load(&mut conn).await?;

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

    async fn add_relation(&self, from: &Uuid, to: &Uuid, relation_type: &str) -> Result<()> {
        use diesel::prelude::*;

        let from_str = from.to_string();
        let to_str = to.to_string();
        let mut conn = self.get_conn().await?;

        // First, ensure the relation table exists
        sql_query(
            r#"
            CREATE TABLE IF NOT EXISTS snippet_relations (
                from_uuid TEXT NOT NULL,
                to_uuid TEXT NOT NULL,
                relation_type TEXT NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (from_uuid, to_uuid, relation_type),
                FOREIGN KEY (from_uuid) REFERENCES snippets(uuid) ON DELETE CASCADE,
                FOREIGN KEY (to_uuid) REFERENCES snippets(uuid) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&mut conn)
        .await
        .map_err(|e| Error::other(format!("Failed to create relations table: {}", e)))?;

        // Add the relation
        let query = r#"
            INSERT INTO snippet_relations (from_uuid, to_uuid, relation_type)
            VALUES ($1, $2, $3)
            ON CONFLICT (from_uuid, to_uuid, relation_type) DO NOTHING
        "#;

        sql_query(query)
            .bind::<Text, _>(&from_str)
            .bind::<Text, _>(&to_str)
            .bind::<Text, _>(relation_type)
            .execute(&mut conn)
            .await
            .map_err(|e| Error::other(format!("Failed to add relation: {}", e)))?;

        Ok(())
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
            "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'snippet_relations')"
        )
        .get_result::<bool>(&mut conn)
        .await
        .unwrap_or(false);

        if !table_exists {
            return Ok(Vec::new());
        }

        // Build the query based on whether we're filtering by relation type
        let (query, params) = if let Some(rel_type) = relation_type {
            (
                "
                SELECT s.* FROM snippets s
                JOIN snippet_relations r ON s.uuid = r.to_uuid
                WHERE r.from_uuid = $1 AND r.relation_type = $2
                ORDER BY r.created_at DESC
                "
                .to_string(),
                vec![
                    Box::new(id_str) as Box<dyn diesel::query_builder::QueryFragment<Pg> + Send>,
                    Box::new(rel_type.to_string()) as _,
                ],
            )
        } else {
            (
                "
                SELECT s.* FROM snippets s
                JOIN snippet_relations r ON s.uuid = r.to_uuid
                WHERE r.from_uuid = $1
                ORDER BY r.created_at DESC
                "
                .to_string(),
                vec![Box::new(id_str) as Box<dyn diesel::query_builder::QueryFragment<Pg> + Send>],
            )
        };

        // Execute the query
        let rows = conn.query(&query, &params[..]).await?;

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
    use crate::{
        database::create_pool,
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
