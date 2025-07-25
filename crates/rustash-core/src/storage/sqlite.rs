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
    sql_query,
    sql_types::{Binary as SqlBinary, Double, Integer as SqlInteger, Nullable, Text, Timestamp},
    SqliteConnection,
};
use diesel_async::{
    pooled_connection::{
        bb8::PooledConnection, AsyncDieselConnectionManager,
    },
    sync_connection_wrapper::SyncConnectionWrapper,
    RunQueryDsl,
};
use std::sync::Arc;
use uuid::Uuid;

type SqlitePool = crate::database::sqlite_pool::SqlitePool;
type SqlitePooledConnection<'a> =
    PooledConnection<'a, AsyncDieselConnectionManager<SyncConnectionWrapper<SqliteConnection>>>;

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
    async fn get_conn(&self) -> Result<SqlitePooledConnection<'_>> {
        self.pool
            .get()
            .await
            .map_err(|e| Error::Pool(e.to_string()))
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
            .first::<DbSnippet>(&mut conn)
            .await
            .optional()
            .map_err(Error::from)?;

        match result {
            Some(snippet) => {
                let with_tags: SnippetWithTags = snippet.into();
                Ok(Some(
                    Box::new(with_tags) as Box<dyn crate::memory::MemoryItem + Send + Sync>,
                ))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        use crate::schema::snippets::dsl::*;

        let id_str = id.to_string();
        let mut conn = self.get_conn().await?;

        diesel::delete(snippets.filter(uuid.eq(id_str)))
            .execute(&mut conn)
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
            #[diesel(sql_type = Nullable<SqlBinary>)]
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
        let query = "SELECT s.*, vs.distance \
                     FROM snippets s \
                     JOIN vss_snippets vs ON s.rowid = vs.rowid \
                     WHERE vss_search(vs.embedding, vss_search_params(?, ?))";

        let results = sql_query(query)
            .bind::<SqlBinary, _>(&embedding_bytes)
            .bind::<SqlInteger, _>(limit as i32)
            .load::<SnippetWithDistance>(&mut conn)
            .await?;

        // Convert the results to the expected format
        let items = results
            .into_iter()
            .map(|row| {
                let with_tags: SnippetWithTags = Snippet {
                    uuid: row.uuid,
                    title: row.title,
                    content: row.content,
                    tags: row.tags,
                    embedding: row.embedding,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                }
                .into();

                (
                    Box::new(with_tags) as Box<dyn crate::memory::MemoryItem + Send + Sync>,
                    row.distance as f32,
                )
            })
            .collect();

        Ok(items)
    }

    async fn add_relation(&self, from: &Uuid, to: &Uuid, relation_type: &str) -> Result<()> {
        let mut conn = self.get_conn().await?;
        diesel::insert_into(relations::table)
            .values((
                relations::from_uuid.eq(from.to_string()),
                relations::to_uuid.eq(to.to_string()),
                relations::relation_type.eq(relation_type),
            ))
            .on_conflict_do_nothing()
            .execute(&mut conn)
            .await?;
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
                .filter(title.like(search_term.clone()))
                .or_filter(content.like(search_term));
        }

        if let Some(tag_list) = &query.tags {
            if !tag_list.is_empty() {
                use diesel::dsl::sql;
                let mut tag_conditions = tag_list
                    .iter()
                    .map(|tag| format!("json_each.value = '{}'", tag.replace('\'', "''")))
                    .collect::<Vec<_>>()
                    .join(" OR ");
                tag_conditions = format!(
                    "uuid IN (SELECT s.uuid FROM snippets s, json_each(s.tags) WHERE {})",
                    tag_conditions
                );

                query_builder =
                    query_builder.filter(sql::<diesel::sql_types::Bool>(&tag_conditions));
            }
        }

        if let Some(limit) = query.limit {
            query_builder = query_builder.limit(limit as i64);
        }

        let results: Vec<DbSnippet> = query_builder
            .load::<DbSnippet>(&mut conn)
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
        let mut conn = self.get_conn().await?;
        let mut query = relations::table
            .inner_join(snippets::table.on(relations::to_uuid.eq(snippets::uuid)))
            .filter(relations::from_uuid.eq(id.to_string()))
            .select(DbSnippet::as_select())
            .into_boxed();

        if let Some(rel_type) = relation_type {
            query = query.filter(relations::relation_type.eq(rel_type));
        }

        let results: Vec<DbSnippet> = query.load(&mut conn).await?;

        let items = results
            .into_iter()
            .map(|s| {
                let with_tags: SnippetWithTags = s.into();
                Box::new(with_tags) as Box<dyn crate::memory::MemoryItem + Send + Sync>
            })
            .collect();
        Ok(items)
    }
}