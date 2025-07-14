//! SQLite backend implementation for Rustash storage.

use super::StorageBackend;
use crate::{
    error::{Error, Result},
    models::{DbSnippet, NewDbSnippet, Query, Snippet, SnippetWithTags},
    schema::{relations, snippets},
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use diesel::{
    prelude::*,
    query_builder::AsQuery,
    query_dsl::methods::LoadQuery,
    sql_query,
    sql_types::{Binary, Integer, Nullable, Text, Timestamp},
};
use diesel_async::{
    pooled_connection::bb8::PooledConnection,
    AsyncConnection,
    RunQueryDsl,
};
use std::sync::Arc;
use uuid::Uuid;

/// A SQLite-backed storage implementation.
#[derive(Debug, Clone)]
pub struct SqliteBackend {
    pool: Arc<crate::database::sqlite_pool::SqlitePool>,
}

impl SqliteBackend {
    /// Create a new SQLite backend with the given connection pool.
    pub fn new(pool: crate::database::sqlite_pool::SqlitePool) -> Self {
        Self { pool: Arc::new(pool) }
    }

    /// Get a connection from the pool.
    async fn get_conn(
        &self,
    ) -> Result<PooledConnection<'_, diesel_async::pooled_connection::AsyncDieselConnectionManager<diesel_async::AsyncSqliteConnection>>> {
        self.pool.get().await.map_err(|e| Error::Pool(e.to_string()))
    }
}

#[async_trait]
impl StorageBackend for SqliteBackend {
    async fn save(&self, item: &(dyn crate::memory::MemoryItem + Send + Sync)) -> Result<()> {
        let snippet = item
            .as_any()
            .downcast_ref::<SnippetWithTags>()
            .ok_or_else(|| Error::other("Invalid item type: Expected SnippetWithTags"))?;
            
        let tags_json = serde_json::to_string(&snippet.tags)?;
        let mut conn = self.get_conn().await?;
        let now = chrono::Utc::now().naive_utc();

        let db_snippet = NewDbSnippet {
            uuid: snippet.uuid.clone(),
            title: snippet.title.clone(),
            content: snippet.content.clone(),
            tags: tags_json,
            embedding: snippet.embedding.clone(),
        };

        conn.transaction(|conn| {
            Box::pin(async move {
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
                Ok(())
            })
        }).await?;

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
        let mut conn = self.get_conn().await?;
        let mut query_builder = snippets.into_boxed();

        if let Some(text_filter) = &query.text_filter {
            let search_term = format!("%{}%", text_filter);
            query_builder = query_builder
                .filter(title.like(&search_term))
                .or_filter(content.like(&search_term));
        }

        if let Some(tag_list) = &query.tags {
            if !tag_list.is_empty() {
                use diesel::dsl::sql;
                let tag_conditions = tag_list
                    .iter()
                    .map(|tag| format!("tags LIKE '%\"{}\"%'", tag.replace("'", "''")))
                    .collect::<Vec<_>>()
                    .join(" OR ");
                let sql_query = format!("({})", tag_conditions);
                query_builder = query_builder.filter(sql::<diesel::sql_types::Bool>(&sql_query));
            }
        }
        
        if let Some(limit) = query.limit {
            query_builder = query_builder.limit(limit as i64);
        }

        let results: Vec<DbSnippet> = query_builder
            .load::<DbSnippet>(&mut *conn)
            .await
            .map_err(Error::from)?;

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
        #[derive(QueryableByName)]
        struct Row {
            #[diesel(sql_type = Text)]
            uuid: String,
            #[diesel(sql_type = Text)]
            title: String,
            #[diesel(sql_type = Text)]
            content: String,
            #[diesel(sql_type = Text)]
            tags: String,
            #[diesel(sql_type = Nullable<Binary>)]
            embedding: Option<Vec<u8>>,
            #[diesel(sql_type = Timestamp)]
            created_at: NaiveDateTime,
            #[diesel(sql_type = Timestamp)]
            updated_at: NaiveDateTime,
        }

        let mut sql = String::from(
            "SELECT s.uuid, s.title, s.content, s.tags, s.embedding, \
             s.created_at, s.updated_at \
             FROM snippets s \
             JOIN relations r ON r.to_uuid = s.uuid \
             WHERE r.from_uuid = ?",
        );
        if relation_type.is_some() {
            sql.push_str(" AND r.relation_type = ?");
        }

        let mut conn = self.get_conn().await?;
        let mut query = diesel::sql_query(sql).bind::<Text, _>(id.to_string());
        if let Some(rel) = relation_type {
            query = query.bind::<Text, _>(rel);
        }

        let rows: Vec<Row> = query.load(&mut *conn).await?;

        let items = rows
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
                Box::new(with_tags) as Box<dyn crate::memory::MemoryItem + Send + Sync>
            })
            .collect();

        Ok(items)
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
    use std::sync::Arc;
    use uuid::Uuid;

    async fn create_test_backend() -> SqliteBackend {
        let pool = create_test_pool().await.unwrap();
        SqliteBackend::new(pool)
    }

    #[tokio::test]
    async fn test_save_and_get() {
        let backend = create_test_backend().await;
        let snippet_id = Uuid::new_v4();
        let snippet = SnippetWithTags {
            uuid: snippet_id.to_string(),
            id: snippet_id,
            title: "Test Snippet".to_string(),
            content: "Test content".to_string(),
            tags: vec!["test".to_string()],
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        backend.save(&snippet).await.unwrap();

        let retrieved = backend.get(&snippet_id).await.unwrap().unwrap();
        let retrieved_snippet = retrieved
            .as_any()
            .downcast_ref::<SnippetWithTags>()
            .unwrap();

        assert_eq!(retrieved_snippet.title, snippet.title);
        assert_eq!(retrieved_snippet.content, snippet.content);
    }

    #[tokio::test]
    async fn test_save_and_update() {
        let backend = create_test_backend().await;

        let snippet_id = Uuid::new_v4();
        let mut snippet = SnippetWithTags {
            uuid: snippet_id.to_string(),
            id: snippet_id,
            title: "Initial Title".to_string(),
            content: "Initial Content".to_string(),
            tags: vec!["initial".to_string()],
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

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

        let snippet1 = SnippetWithTags {
            uuid: Uuid::new_v4().to_string(),
            id: Uuid::new_v4(),
            title: "Python List".to_string(),
            content: "my_list = [1, 2, 3]".to_string(),
            tags: vec!["python".to_string()],
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let snippet2 = SnippetWithTags {
            uuid: Uuid::new_v4().to_string(),
            id: Uuid::new_v4(),
            title: "Rust Vector".to_string(),
            content: "let vec = vec![1, 2, 3];".to_string(),
            tags: vec!["rust".to_string()],
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        backend.save(&snippet1).await.unwrap();
        backend.save(&snippet2).await.unwrap();

        let query = crate::models::Query {
            text_filter: Some("vector".to_string()),
            tags: None,
            limit: Some(10),
            ..Default::default()
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
    async fn test_get_related() {
        let backend = create_test_backend().await;

        let source = SnippetWithTags {
            uuid: Uuid::new_v4().to_string(),
            id: Uuid::new_v4(),
            title: "Source".to_string(),
            content: "source".to_string(),
            tags: vec![],
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let target = SnippetWithTags {
            uuid: Uuid::new_v4().to_string(),
            id: Uuid::new_v4(),
            title: "Target".to_string(),
            content: "target".to_string(),
            tags: vec![],
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        backend.save(&source).await.unwrap();
        backend.save(&target).await.unwrap();

        backend
            .add_relation(&source.id, &target.id, "related")
            .await
            .unwrap();

        let results = backend
            .get_related(&source.id, Some("related"))
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        let related = results[0]
            .as_any()
            .downcast_ref::<SnippetWithTags>()
            .unwrap();
        assert_eq!(related.title, "Target");
    }
}
