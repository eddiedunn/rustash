//! SQLite backend implementation for Rustash storage.

use super::StorageBackend;
use crate::database::connection_pool::DatabaseConnection;
use crate::error::Error;
use crate::{
    database::SqlitePool,
    error::Result,
    models::{NewDbSnippet, Query, Snippet, SnippetWithTags},
};
use async_trait::async_trait;
use diesel::prelude::*;
use diesel::sql_query;
use diesel_async::{AsyncSqliteConnection, RunQueryDsl};

use std::sync::Arc;

/// A SQLite-backed storage implementation.
#[derive(Debug, Clone)]
pub struct SqliteBackend {
    pool: Arc<SqlitePool>,
}

impl SqliteBackend {
    /// Create a new SQLite backend with the given connection pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    /// Get a connection from the pool.
    async fn get_conn(
        &self,
    ) -> Result<bb8::PooledConnection<'_, <AsyncSqliteConnection as DatabaseConnection>::Manager>>
    {
        self.pool.get_connection().await.map_err(Into::into)
    }
}

#[async_trait]
impl StorageBackend for SqliteBackend {
    async fn save(&self, item: &(dyn crate::memory::MemoryItem + Send + Sync)) -> Result<()> {
        use diesel_async::RunQueryDsl;

        let snippet = item
            .as_any()
            .downcast_ref::<SnippetWithTags>()
            .ok_or_else(|| Error::other("Invalid item type"))?;

        let new_snippet = NewDbSnippet::new(
            snippet.title.clone(),
            snippet.content.clone(),
            snippet.tags.clone(),
        );

        // Convert to the database model
        let db_snippet: NewDbSnippet = new_snippet.into();

        let mut conn = self.get_conn().await?;

        // Check if the snippet already exists
        let existing: Option<Snippet> = crate::schema::snippets::table
            .filter(crate::schema::snippets::uuid.eq(&db_snippet.uuid))
            .first(&mut *conn)
            .await
            .optional()
            .map_err(Error::from)?;

        if let Some(_) = existing {
            // Update existing snippet
            diesel::update(crate::schema::snippets::table)
                .filter(crate::schema::snippets::uuid.eq(&db_snippet.uuid))
                .set((
                    crate::schema::snippets::title.eq(&db_snippet.title),
                    crate::schema::snippets::content.eq(&db_snippet.content),
                    crate::schema::snippets::tags.eq(&db_snippet.tags),
                    crate::schema::snippets::updated_at.eq(chrono::Utc::now().naive_utc()),
                ))
                .execute(&mut *conn)
                .await
                .map_err(Error::from)?;
        } else {
            // Insert new snippet
            diesel::insert_into(crate::schema::snippets::table)
                .values(&db_snippet)
                .execute(&mut *conn)
                .await
                .map_err(Error::from)?;
        }

        Ok(())
    }

    async fn get(
        &self,
        id: &Uuid,
    ) -> Result<Option<Box<dyn crate::memory::MemoryItem + Send + Sync>>> {
        use crate::schema::snippets::dsl::*;
        use diesel_async::RunQueryDsl;

        let id_str = id.to_string();
        let mut conn = self.get_conn().await?;

        let result: Option<Snippet> = snippets
            .filter(uuid.eq(&id_str))
            .first::<Snippet>(&mut *conn)
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
        use crate::schema::snippets::dsl::*;
        use diesel_async::RunQueryDsl;

        let id_str = id.to_string();
        let mut conn = self.get_conn().await?;

        diesel::delete(snippets.filter(uuid.eq(id_str)))
            .execute(&mut *conn)
            .await
            .map_err(Error::from)?;

        Ok(())
    }

    async fn vector_search(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(Box<dyn crate::memory::MemoryItem + Send + Sync>, f32)>> {
        // SQLite VSS requires a bincode-serialized, f32 little-endian vector.
        let embedding_bytes = bincode::serialize(embedding)?;

        // The query finds the rowids of the N nearest neighbors from the VSS table,
        // then joins back to the snippets table to get the full snippet data.
        let query = diesel::sql_query(
            "SELECT s.* FROM snippets s JOIN vss_snippets vs ON s.rowid = vs.rowid
             WHERE vs.vss_search(?, ?)
             ORDER BY vs.distance",
        )
        .bind::<diesel::sql_types::Blob, _>(&embedding_bytes)
        .bind::<diesel::sql_types::Integer, _>(limit as i32);

        let mut conn = self.get_conn().await?;
        let snippets: Vec<Snippet> = query.load(&mut *conn).await?;

        // Note: The distance is not easily retrievable in the same query.
        // For this implementation, we return a dummy distance of 0.0.
        // A more complex query could retrieve it if needed.
        let results = snippets
            .into_iter()
            .map(|s| {
                let with_tags: SnippetWithTags = s.into();
                (
                    Box::new(with_tags) as Box<dyn crate::memory::MemoryItem + Send + Sync>,
                    0.0,
                )
            })
            .collect();

        Ok(results)
    }

    async fn add_relation(&self, from: &Uuid, to: &Uuid, relation_type: &str) -> Result<()> {
        use diesel::sql_types::Text;
        use diesel_async::RunQueryDsl;

        let mut conn = self.get_conn().await?;

        sql_query(
            "CREATE TABLE IF NOT EXISTS relations (from_uuid TEXT NOT NULL, to_uuid TEXT NOT NULL, relation_type TEXT NOT NULL)",
        )
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?;

        sql_query("INSERT INTO relations (from_uuid, to_uuid, relation_type) VALUES (?1, ?2, ?3)")
            .bind::<Text, _>(from.to_string())
            .bind::<Text, _>(to.to_string())
            .bind::<Text, _>(relation_type)
            .execute(&mut *conn)
            .await
            .map_err(Error::from)?;

        Ok(())
    }

    async fn query(
        &self,
        query: &Query,
    ) -> Result<Vec<Box<dyn crate::memory::MemoryItem + Send + Sync>>> {
        use crate::schema::snippets::dsl::*;
        use diesel_async::RunQueryDsl;

        let query_text = query.text_filter.clone().unwrap_or_default();
        let query_limit = query.limit.unwrap_or(10) as i64;

        let mut conn = self.get_conn().await?;

        // Start building the query
        let mut query_builder = snippets.into_boxed();

        // Apply text filter if provided
        if !query_text.is_empty() {
            query_builder = query_builder.filter(
                title
                    .like(format!("%{}%", query_text))
                    .or(content.like(format!("%{}%", query_text)))
                    .or(tags.like(format!("%{}%", query_text))),
            );
        }

        // Apply sorting
        query_builder = match query.sort_by.as_deref() {
            Some("title") => query_builder.order(title.asc()),
            Some("created_at") => query_builder.order(created_at.desc()),
            Some("updated_at") => query_builder.order(updated_at.desc()),
            _ => query_builder.order(created_at.desc()),
        };

        // Apply limit
        query_builder = query_builder.limit(query_limit);

        // Execute the query
        let results: Vec<Snippet> = query_builder
            .load::<Snippet>(&mut *conn)
            .await
            .map_err(Error::from)?;

        // Convert to MemoryItems
        let items = results
            .into_iter()
            .map(|s| {
                let with_tags: SnippetWithTags = s.into();
                Box::new(with_tags) as Box<dyn crate::memory::MemoryItem + Send + Sync>
            })
            .collect();

        Ok(items)
    }

    async fn get_related(
        &self,
        id: &Uuid,
        relation_type: Option<&str>,
    ) -> Result<Vec<Box<dyn crate::memory::MemoryItem + Send + Sync>>> {
        use diesel::sql_types::Text;
        use diesel_async::RunQueryDsl;

        let mut conn = self.get_conn().await?;

        sql_query(
            "CREATE TABLE IF NOT EXISTS relations (from_uuid TEXT NOT NULL, to_uuid TEXT NOT NULL, relation_type TEXT NOT NULL)",
        )
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?;

        let query = if let Some(rel) = relation_type {
            sql_query(
                "SELECT s.* FROM snippets s JOIN relations r ON s.uuid = r.to_uuid WHERE r.from_uuid = ?1 AND r.relation_type = ?2",
            )
            .bind::<Text, _>(id.to_string())
            .bind::<Text, _>(rel)
        } else {
            sql_query(
                "SELECT s.* FROM snippets s JOIN relations r ON s.uuid = r.to_uuid WHERE r.from_uuid = ?1",
            )
            .bind::<Text, _>(id.to_string())
        };

        let rows = query
            .load::<Snippet>(&mut *conn)
            .await
            .map_err(Error::from)?;

        Ok(rows
            .into_iter()
            .map(|s| {
                let with_tags: SnippetWithTags = s.into();
                Box::new(with_tags) as Box<dyn crate::memory::MemoryItem + Send + Sync>
            })
            .collect())
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
        let pool = create_pool(database_url)
            .await
            .expect("Failed to create test pool");

        // Get a connection from the pool to run migrations
        let mut conn = pool
            .get()
            .await
            .expect("Failed to get connection from pool");

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
