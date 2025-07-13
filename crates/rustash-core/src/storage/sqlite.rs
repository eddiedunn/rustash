//! SQLite backend implementation for Rustash storage.

use super::StorageBackend;
use crate::{
    database::sqlite_pool::SqlitePool,
    error::{Error, Result},
    models::{Query, Snippet, SnippetWithTags},
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use diesel::{
    prelude::*,
    query_builder::AsQuery,
    query_dsl::methods::LoadQuery,
    sql_query,
    sql_types::{Binary, Integer},
    row::NamedRow,
};
use diesel_async::{
    pooled_connection::bb8::PooledConnection,
    AsyncConnection,
    RunQueryDsl,
};
use diesel_async::sqlite::{SqliteRow, SqliteValue, AsyncSqliteConnection};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

/// A SQLite-backed storage implementation.
#[derive(Debug, Clone)]
pub struct SqliteBackend {
    pool: std::sync::Arc<SqlitePool>,
}

impl SqliteBackend {
    /// Create a new SQLite backend with the given connection pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool: std::sync::Arc::new(pool) }
    }

    /// Get a connection from the pool.
    async fn get_conn(&self) -> Result<PooledConnection<'_, diesel_async::pooled_connection::AsyncDieselConnectionManager<AsyncSqliteConnection>>> {
    self.pool.get().await.map_err(|e| Error::Pool(e.to_string()))
}

    /// Convert a database row to a Snippet
    fn row_to_snippet(&self, row: &SqliteRow) -> Result<Snippet> {
        let uuid: String = row.get("uuid").map(|v: &SqliteValue| v.as_str().unwrap_or_default().to_string())?;
        let title: String = row.get("title").map(|v: &SqliteValue| v.as_str().unwrap_or_default().to_string())?;
        let content: String = row.get("content").map(|v: &SqliteValue| v.as_str().unwrap_or_default().to_string())?;
        let tags_json: String = row.get("tags").map(|v: &SqliteValue| v.as_str().unwrap_or_default().to_string())?;
        let embedding: Option<Vec<u8>> = row.get("embedding").map(|v: &SqliteValue| v.as_bytes().map(|b| b.to_vec())).transpose()?;
        let created_at: NaiveDateTime = row.get("created_at").and_then(|v: &SqliteValue| {
            v.as_str().and_then(|s| NaiveDateTime::from_str(s).ok())
        }).ok_or_else(|| Error::other("Invalid created_at timestamp"))?;
        let updated_at: NaiveDateTime = row.get("updated_at").and_then(|v: &SqliteValue| {
            v.as_str().and_then(|s| NaiveDateTime::from_str(s).ok())
        }).ok_or_else(|| Error::other("Invalid updated_at timestamp"))?;

        // Validate the UUID format
        Uuid::parse_str(&uuid).map_err(Error::from)?;

        // Parse tags from JSON
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

        Ok(Snippet {
            uuid,
            title,
            content,
            tags: tags_json,  // Use the raw JSON string as expected by the Snippet struct
            embedding,
            created_at,
            updated_at,
        })
    }
}

#[async_trait]
impl StorageBackend for SqliteBackend {
    async fn save(&self, item: &(dyn crate::memory::MemoryItem + Send + Sync)) -> Result<()> {
        use diesel_async::RunQueryDsl;

        let snippet = item
            .as_any()
            .downcast_ref::<SnippetWithTags>()
            .ok_or_else(|| Error::other("Invalid item type: Expected SnippetWithTags"))?;

        let db_snippet = NewDbSnippet {
            uuid: snippet.uuid.clone(),
            title: snippet.title.clone(),
            content: snippet.content.clone(),
            tags: serde_json::to_string(&snippet.tags).unwrap_or_else(|_| "[]".to_string()),
            embedding: snippet.embedding.clone(),
        };

        let mut conn = self.get_conn().await?;

        diesel::insert_into(crate::schema::snippets::table)
            .values(&db_snippet)
            .on_conflict(crate::schema::snippets::uuid)
            .do_update()
            .set((
                crate::schema::snippets::title.eq(&db_snippet.title),
                crate::schema::snippets::content.eq(&db_snippet.content),
                crate::schema::snippets::tags.eq(&db_snippet.tags),
                crate::schema::snippets::updated_at.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(&mut *conn)
            .await
            .map_err(Error::from)?;
        Ok(())
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
        use diesel::sql_types::{Binary as SqlBinary, Integer as SqlInteger, Nullable, Text, Timestamp, Double};
        use diesel_async::RunQueryDsl;

        // SQLite VSS requires a bincode-serialized, f32 little-endian vector.
        let embedding_bytes = bincode::serialize(embedding)?;

        // Define a custom type that matches the structure of our query result
        #[derive(QueryableByName)]
        struct SnippetWithDistance {
            #[diesel(sql_type = Text)]
            pub uuid: String,
            #[diesel(sql_type = Text)]
            pub title: String,
            #[diesel(sql_type = Text)]
            pub content: String,
            #[diesel(sql_type = Text)]
            pub tags: String,
            #[diesel(sql_type = Nullable<diesel::sql_types::Binary>)]
            pub embedding: Option<Vec<u8>>,
            #[diesel(sql_type = Timestamp)]
            pub created_at: NaiveDateTime,
            #[diesel(sql_type = Timestamp)]
            pub updated_at: NaiveDateTime,
            #[diesel(sql_type = Double)]
            pub distance: f64,
        }

        let mut conn = self.get_conn().await?;
        
        // Build and execute the raw SQL query with parameters
        let query = format!(
            "SELECT s.uuid, s.title, s.content, s.tags, s.embedding, s.created_at, s.updated_at, vs.distance 
             FROM snippets s 
             JOIN vss_snippets vs ON s.rowid = vs.rowid
             WHERE vss_search(vs.embedding, vss_search_params(?, ?))"
        );
        
        let results = sql_query(&query)
            .bind::<SqlBinary, _>(&embedding_bytes)
            .bind::<SqlInteger, _>(limit as i32)
            .load::<SnippetWithDistance>(&mut *conn)
            .await?;

        // Convert the results to the expected format
        let items = results
            .into_iter()
            .map(|row| {
                let snippet = Snippet {
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
                    row.distance as f32,
                )
            })
            .collect();

        Ok(items)
    }

    async fn add_relation(&self, from: &Uuid, to: &Uuid, relation_type: &str) -> Result<()> {
        let query = diesel::sql_query(
            "INSERT OR IGNORE INTO relations (from_uuid, to_uuid, relation_type) VALUES (?, ?, ?)",
        )
        .bind::<diesel::sql_types::Text, _>(from.to_string())
        .bind::<diesel::sql_types::Text, _>(to.to_string())
        .bind::<diesel::sql_types::Text, _>(relation_type);
        let mut conn = self.get_conn().await?;
        query.execute(&mut *conn).await?;
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
        let mut conn = self.get_conn().await?;
        let mut query_builder = diesel::sql_query(
            "SELECT s.* FROM snippets s JOIN relations r ON s.uuid = r.to_uuid WHERE r.from_uuid = ?",
        )
        .bind::<diesel::sql_types::Text, _>(id.to_string());

        if let Some(rel_type) = relation_type {
            query_builder = query_builder
                .sql(" AND r.relation_type = ?")
                .bind::<diesel::sql_types::Text, _>(rel_type);
        }

        let snippets: Vec<Snippet> = query_builder.load(&mut conn).await?;
        let results = snippets
            .into_iter()
            .map(|s| Box::new(s) as Box<dyn crate::memory::MemoryItem + Send + Sync>)
            .collect();
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{database::sqlite_pool, models::Snippet};
    use chrono::Utc;
    use uuid::Uuid;

    async fn create_test_backend() -> SqliteBackend {
        let pool = sqlite_pool::create_pool(":memory:")
            .await
            .expect("Failed to create test pool");
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

        backend.save(&snippet).await.unwrap();

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
    async fn test_save_and_update() {
        let backend = create_test_backend().await;

        let snippet_id = Uuid::new_v4();
        let mut snippet = Snippet::with_uuid(
            snippet_id,
            "Initial Title".to_string(),
            "Initial Content".to_string(),
            vec!["initial".to_string()],
        );

        backend.save(&snippet).await.unwrap();

        snippet.title = "Updated Title".to_string();
        snippet.content = "Updated Content".to_string();

        backend.save(&snippet).await.unwrap();

        let retrieved = backend.get(&snippet_id).await.unwrap().unwrap();
        let retrieved_snippet = retrieved
            .as_any()
            .downcast_ref::<SnippetWithTags>()
            .unwrap();

        assert_eq!(retrieved_snippet.title, "Updated Title");
        assert_eq!(retrieved_snippet.content, "Updated Content");
    }

    #[tokio::test]
    async fn test_query() {
        let backend = create_test_backend().await;

        let snippet1 = Snippet::with_uuid(
            Uuid::new_v4(),
            "Python Code".to_string(),
            "print('hello')".to_string(),
            vec!["python".to_string()],
        );
        let snippet2 = Snippet::with_uuid(
            Uuid::new_v4(),
            "Rust Code".to_string(),
            "println!(\"hello\")".to_string(),
            vec!["rust".to_string()],
        );

        backend.save(&snippet1).await.unwrap();
        backend.save(&snippet2).await.unwrap();

        let query = Query {
            text_filter: Some("Rust".to_string()),
            ..Default::default()
        };

        let results = backend.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        let result = results[0].as_any().downcast_ref::<SnippetWithTags>().unwrap();
        assert_eq!(result.title, "Rust Code");
    }
}
