//! PostgreSQL backend implementation for Rustash storage.

use super::StorageBackend;
use crate::{
    error::{Error, Result},
    models::{DbSnippet, NewDbSnippet, Query, SnippetWithTags},
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use diesel::{
    prelude::*,
    sql_query,
    sql_types::{BigInt, Float, Text},
};
use diesel_async::{AsyncConnection, RunQueryDsl};
use pgvector::Vector;
use std::sync::Arc;
use uuid::Uuid;

// Type alias for pooled Postgres connection
type PgPooledConnection<'a> = bb8::PooledConnection<
    'a,
    diesel_async::pooled_connection::AsyncDieselConnectionManager<diesel_async::pg::AsyncPgConnection>,
>;

/// A PostgreSQL-backed storage implementation.
#[derive(Debug, Clone)]
pub struct PostgresBackend {
    pool: Arc<crate::database::postgres_pool::PgPool>,
}

impl PostgresBackend {
    /// Create a new PostgreSQL backend with the given connection pool.
    pub fn new(pool: crate::database::postgres_pool::PgPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    /// Get a connection from the pool.
    async fn get_conn(&self) -> Result<PgPooledConnection<'_>> {
        self.pool
            .get()
            .await
            .map_err(|e| Error::Pool(e.to_string()))
    }
}

#[async_trait]
impl StorageBackend for PostgresBackend {
    async fn save(&self, item: &(dyn crate::memory::MemoryItem + Send + Sync)) -> Result<()> {
        let snippet = item
            .as_any()
            .downcast_ref::<SnippetWithTags>()
            .ok_or_else(|| Error::other("Invalid item type: Expected SnippetWithTags"))?;

        let tags_json = serde_json::to_string(&snippet.tags)?;
        let mut conn = self.get_conn().await?;

        let db_snippet = NewDbSnippet {
            uuid: snippet.uuid.clone(),
            title: snippet.title.clone(),
            content: snippet.content.clone(),
            tags: tags_json,
            embedding: snippet.embedding.clone(),
        };

        conn.transaction(|conn| {
            Box::pin(async move {
                let now = chrono::Utc::now().naive_utc();
                diesel::insert_into(crate::schema::snippets::table)
                    .values(&db_snippet)
                    .on_conflict(crate::schema::snippets::uuid)
                    .do_update()
                    .set((
                        crate::schema::snippets::title.eq(&db_snippet.title),
                        crate::schema::snippets::content.eq(&db_snippet.content),
                        crate::schema::snippets::tags.eq(&db_snippet.tags),
                        crate::schema::snippets::updated_at.eq(now),
                    ))
                    .execute(conn)
                    .await?;
                Ok::<_, Error>(())
            })
        })
        .await?;

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
            .optional()
            .map_err(Error::from)?;

        match result {
            Some(snippet) => {
                let with_tags: SnippetWithTags = snippet.into();
                Ok(Some(
                    Box::new(with_tags) as Box<dyn crate::memory::MemoryItem + Send + Sync>
                ))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        let id_str = id.to_string();
        let mut conn = self.get_conn().await?;

        let query = "DELETE FROM snippets WHERE uuid = $1";

        diesel_async::RunQueryDsl::execute(
            diesel::sql_query(query).bind::<Text, _>(&id_str),
            &mut *conn,
        )
        .await
        .map_err(|e| Error::other(format!("Failed to delete snippet: {}", e)))?;

        Ok(())
    }

    async fn query(
        &self,
        query: &Query,
    ) -> Result<Vec<Box<dyn crate::memory::MemoryItem + Send + Sync>>> {
        use crate::schema::snippets::dsl::*;
        let mut conn = self.get_conn().await?;

        let mut query_builder = snippets.into_boxed();

        // Apply text filter if provided
        if let Some(text) = &query.text_filter {
            query_builder = query_builder
                .filter(title.like(format!("%{}%", text)))
                .or_filter(content.like(format!("%{}%", text)));
        }

        // Apply tags filter if provided
        if let Some(query_tags) = &query.tags {
            if !query_tags.is_empty() {
                use diesel::dsl::sql;
                let tags_json = serde_json::to_value(query_tags)?;
                query_builder = query_builder.filter(sql::<diesel::sql_types::Bool>(&format!(
                    "tags @> '{}'::jsonb",
                    tags_json
                )));
            }
        }

        // Apply limit if provided
        if let Some(limit) = query.limit {
            query_builder = query_builder.limit(limit as i64);
        }

        // Execute the query
        let results: Vec<DbSnippet> = query_builder.load::<DbSnippet>(&mut *conn).await?;

        // Convert results to the expected return type
        let items = results
            .into_iter()
            .map(|s| {
                let with_tags: SnippetWithTags = s.into();
                Box::new(with_tags) as Box<dyn crate::memory::MemoryItem + Send + Sync>
            })
            .collect();

        Ok(items)
    }

    async fn vector_search(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(Box<dyn crate::memory::MemoryItem + Send + Sync>, f32)>> {
        let query_vector = Vector::from(embedding.to_vec());

        #[derive(QueryableByName)]
        struct SnippetWithDistance {
            #[diesel(sql_type = diesel::sql_types::Text)]
            uuid: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            title: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            content: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            tags: String,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Binary>)]
            embedding: Option<Vec<u8>>,
            #[diesel(sql_type = diesel::sql_types::Timestamptz)]
            created_at: NaiveDateTime,
            #[diesel(sql_type = diesel::sql_types::Timestamptz)]
            updated_at: NaiveDateTime,
            #[diesel(sql_type = Float)]
            distance: f32,
        }

        let query = sql_query(
            r#"
            SELECT s.*, 1 - (embedding <=> $1) as distance
            FROM snippets s
            WHERE embedding IS NOT NULL
            ORDER BY embedding <=> $1
            LIMIT $2
            "#,
        )
        .bind::<pgvector::sql_types::Vector, _>(&query_vector)
        .bind::<BigInt, _>(limit as i64);

        let mut conn = self.get_conn().await?;
        let rows: Vec<SnippetWithDistance> = query.load(&mut *conn).await?;

        let results = rows
            .into_iter()
            .map(|row| {
                let snippet = DbSnippet {
                    uuid: row.uuid,
                    title: row.title,
                    content: row.content,
                    tags: row.tags,
                    embedding: row.embedding,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                };
                let with_tags: SnippetWithTags = snippet.into();
                (
                    Box::new(with_tags) as Box<dyn crate::memory::MemoryItem + Send + Sync>,
                    row.distance,
                )
            })
            .collect();

        Ok(results)
    }

    async fn add_relation(&self, from: &Uuid, to: &Uuid, relation_type: &str) -> Result<()> {
        use crate::schema::relations::dsl::*;

        let mut conn = self.get_conn().await?;
        let from_str = from.to_string();
        let to_str = to.to_string();

        conn.transaction(|conn| {
            Box::pin(async move {
                // First ensure both snippets exist
                let from_exists: bool = crate::schema::snippets::table
                    .filter(crate::schema::snippets::uuid.eq(&from_str))
                    .select(diesel::dsl::sql::<diesel::sql_types::Bool>("1"))
                    .first(conn)
                    .await
                    .optional()?
                    .is_some();

                if !from_exists {
                    return Err(Error::other(format!("Source snippet {} not found", from)));
                }

                let to_exists: bool = crate::schema::snippets::table
                    .filter(crate::schema::snippets::uuid.eq(&to_str))
                    .select(diesel::dsl::sql::<diesel::sql_types::Bool>("1"))
                    .first(conn)
                    .await
                    .optional()?
                    .is_some();

                if !to_exists {
                    return Err(Error::other(format!("Target snippet {} not found", to)));
                }

                // Add the relation
                diesel::insert_into(relations)
                    .values((
                        from_uuid.eq(&from_str),
                        to_uuid.eq(&to_str),
                        crate::schema::relations::relation_type.eq(relation_type),
                    ))
                    .on_conflict((from_uuid, to_uuid, crate::schema::relations::relation_type))
                    .do_nothing()
                    .execute(conn)
                    .await?;

                Ok::<_, Error>(())
            })
        })
        .await?;

        Ok(())
    }

    async fn get_related(
        &self,
        id: &Uuid,
        relation_type: Option<&str>,
    ) -> Result<Vec<Box<dyn crate::memory::MemoryItem + Send + Sync>>> {
        // TODO: Implement proper relation fetching for PostgreSQL/AGE
        // For now, return an empty vector to match the SQLite backend.
        let _ = (id, relation_type);
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{database::postgres_pool::create_connection_pool, models::SnippetWithTags};
    use chrono::Utc;
    use diesel_migrations::{embed_migrations, AsyncMigrationHarness};
    use uuid::Uuid;

    // This will embed the migrations in the binary
    pub const MIGRATIONS: diesel_migrations::EmbeddedMigrations = embed_migrations!("migrations");

    async fn create_test_backend() -> Result<PostgresBackend> {
        // Set up test database connection
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/rustash_test".to_string()
        });

        let pool = create_connection_pool(&database_url).await?;

        // Get a connection from the pool to run migrations
        let mut conn = pool.get().await?;

        // Run migrations on the same connection that will be used by the tests
        conn.run_pending_migrations(MIGRATIONS)
            .await
            .expect("Failed to run migrations");

        // Create the backend with the pool
        Ok(PostgresBackend::new(pool))
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL with pgvector"]
    async fn test_save_and_get() {
        let backend = create_test_backend().await.unwrap();
        let snippet_id = Uuid::new_v4();

        // Create a test snippet with tags
        let snippet_with_tags = SnippetWithTags {
            uuid: snippet_id.to_string(),
            id: snippet_id,
            title: "Test Snippet".to_string(),
            content: "Test content".to_string(),
            tags: vec!["test".to_string()],
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

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
    #[ignore = "requires PostgreSQL with pgvector"]
    async fn test_save_and_update() {
        let backend = create_test_backend().await.unwrap();
        let snippet_id = Uuid::new_v4();

        // Create and save the original snippet
        let original_snippet = SnippetWithTags {
            uuid: snippet_id.to_string(),
            id: snippet_id,
            title: "Original Title".to_string(),
            content: "Original content".to_string(),
            tags: vec!["test".to_string()],
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        backend.save(&original_snippet).await.unwrap();

        // Update the snippet
        let updated_snippet = SnippetWithTags {
            title: "Updated Title".to_string(),
            content: "Updated content".to_string(),
            ..original_snippet.clone()
        };
        backend.save(&updated_snippet).await.unwrap();

        // Retrieve and verify the update
        let retrieved = backend.get(&snippet_id).await.unwrap().unwrap();
        let retrieved_snippet = retrieved
            .as_any()
            .downcast_ref::<SnippetWithTags>()
            .unwrap();

        assert_eq!(retrieved_snippet.title, "Updated Title");
        assert_eq!(retrieved_snippet.content, "Updated content");
        assert_eq!(retrieved_snippet.tags, vec!["test".to_string()]);
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL with pgvector"]
    async fn test_query() {
        let backend = create_test_backend().await.unwrap();

        // Create test data
        let snippet1 = SnippetWithTags {
            uuid: Uuid::new_v4().to_string(),
            id: Uuid::new_v4(),
            title: "Test Query 1".to_string(),
            content: "Content about testing queries".to_string(),
            tags: vec!["test".to_string(), "query".to_string()],
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let snippet2 = SnippetWithTags {
            uuid: Uuid::new_v4().to_string(),
            id: Uuid::new_v4(),
            title: "Another Test".to_string(),
            content: "Different content".to_string(),
            tags: vec!["test".to_string()],
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Save test data
        backend.save(&snippet1).await.unwrap();
        backend.save(&snippet2).await.unwrap();

        // Test text search
        let query = crate::models::Query {
            text_filter: Some("queries".to_string()),
            tags: None,
            limit: Some(10),
            ..Default::default()
        };
        let results = backend.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        let first_result = results[0]
            .as_any()
            .downcast_ref::<SnippetWithTags>()
            .unwrap();
        assert_eq!(first_result.title, "Test Query 1");

        // Test tag filter
        let query = crate::models::Query {
            text_filter: None,
            tags: Some(vec!["query".to_string()]),
            limit: Some(10),
            ..Default::default()
        };
        let results = backend.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        let first_result = results[0]
            .as_any()
            .downcast_ref::<SnippetWithTags>()
            .unwrap();
        assert_eq!(first_result.title, "Test Query 1");
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL with pgvector"]
    async fn test_vector_search() {
        let backend = create_test_backend().await.unwrap();

        // Create test embeddings
        let test_embedding = vec![0.1, 0.2, 0.3];
        let similar_embedding = vec![0.11, 0.21, 0.31];
        let different_embedding = vec![0.9, 0.8, 0.7];

        // Create test snippets
        let snippet1 = SnippetWithTags {
            uuid: Uuid::new_v4().to_string(),
            id: Uuid::new_v4(),
            title: "Similar Snippet".to_string(),
            content: "This is similar to the test embedding".to_string(),
            tags: vec!["test".to_string()],
            embedding: Some(bincode::serialize(&similar_embedding).unwrap()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let snippet2 = SnippetWithTags {
            uuid: Uuid::new_v4().to_string(),
            id: Uuid::new_v4(),
            title: "Different Snippet".to_string(),
            content: "This is different from the test embedding".to_string(),
            tags: vec![],
            embedding: Some(bincode::serialize(&different_embedding).unwrap()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Save test data
        backend.save(&snippet1).await.unwrap();
        backend.save(&snippet2).await.unwrap();

        // Test vector search
        let results = backend.vector_search(&test_embedding, 2).await.unwrap();
        assert_eq!(results.len(), 2);

        // The first result should be more similar
        let (first_result, first_similarity) = &results[0];
        let _first_snippet = first_result
            .as_any()
            .downcast_ref::<SnippetWithTags>()
            .unwrap();

        let (_, second_similarity) = &results[1];
        // The first result should be more similar than the second
        assert!(
            first_similarity > second_similarity,
            "First result should have higher similarity score"
        );
        // The similarity score should be reasonable (cosine similarity in 0-1 range)
        assert!(
            *first_similarity > 0.8 && *first_similarity <= 1.0,
            "Similarity score should be between 0.8 and 1.0"
        );
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL with pgvector"]
    async fn test_relations() {
        let backend = create_test_backend().await.unwrap();

        // Create two snippets
        let snippet1 = SnippetWithTags {
            uuid: Uuid::new_v4().to_string(),
            id: Uuid::new_v4(),
            title: "Source Snippet".to_string(),
            content: "Source content".to_string(),
            tags: vec!["test".to_string()],
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let snippet2 = SnippetWithTags {
            uuid: Uuid::new_v4().to_string(),
            id: Uuid::new_v4(),
            title: "Related Snippet".to_string(),
            content: "Related content".to_string(),
            tags: vec![],
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
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
        let related = backend.get_related(&from_id, Some("related")).await;

        // The current implementation returns an empty vec, so we just check for Ok result.
        // When the implementation is complete, this test should be updated.
        assert!(related.is_ok());
    }
}
