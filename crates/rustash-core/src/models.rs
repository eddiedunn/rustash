//! Data models for Rustash

use crate::memory::MemoryItem;
use crate::schema::snippets;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

/// A snippet stored in the database
#[derive(Queryable, Selectable, Serialize, Deserialize, Debug, Clone, PartialEq, QueryableByName)]
#[diesel(table_name = snippets)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DbSnippet {
    pub id: i32,
    pub uuid: String, // UUID stored as string
    pub title: String,
    pub content: String,
    pub tags: String, // JSON array stored as string
    pub embedding: Option<Vec<u8>>, // Vector embedding as binary
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// A new snippet to be inserted into the database
#[derive(Insertable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = snippets)]
pub struct NewDbSnippet {
    pub uuid: String,
    pub title: String,
    pub content: String,
    pub tags: String, // JSON array stored as string
    pub embedding: Option<Vec<u8>>,
}

/// A lightweight representation of a snippet for list views
#[derive(Queryable, Selectable, Serialize, Deserialize, Debug, Clone, PartialEq, QueryableByName)]
#[diesel(table_name = snippets)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SnippetListItem {
    pub id: i32,
    pub uuid: String,
    pub title: String,
    pub tags: String, // JSON array stored as string
    pub updated_at: NaiveDateTime,
}

/// A snippet with parsed tags for easier handling
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SnippetWithTags {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>, // Parsed from JSON
    pub embedding: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// The main Snippet struct that implements MemoryItem
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Snippet {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub embedding: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MemoryItem for Snippet {
    fn id(&self) -> Uuid { self.id }
    
    fn item_type(&self) -> &'static str { "snippet" }
    
    fn content(&self) -> &str { &self.content }
    
    fn metadata(&self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        map.insert("title".to_string(), Value::String(self.title.clone()));
        map.insert("tags".to_string(), json!(self.tags));
        if let Some(embedding) = &self.embedding {
            map.insert("has_embedding".to_string(), Value::Bool(true));
        }
        map
    }
    
    fn created_at(&self) -> DateTime<Utc> { self.created_at }
    
    fn updated_at(&self) -> DateTime<Utc> { self.updated_at }
}

// Conversion implementations

impl From<DbSnippet> for Snippet {
    fn from(db_snippet: DbSnippet) -> Self {
        let tags: Vec<String> = serde_json::from_str(&db_snippet.tags).unwrap_or_default();
        
        Self {
            id: Uuid::parse_str(&db_snippet.uuid).unwrap_or_else(|_| Uuid::new_v4()),
            title: db_snippet.title,
            content: db_snippet.content,
            tags,
            embedding: db_snippet.embedding,
            created_at: DateTime::<Utc>::from_utc(db_snippet.created_at, Utc),
            updated_at: DateTime::<Utc>::from_utc(db_snippet.updated_at, Utc),
        }
    }
}

impl From<Snippet> for NewDbSnippet {
    fn from(snippet: Snippet) -> Self {
        let tags_json = serde_json::to_string(&snippet.tags).unwrap_or_else(|_| "[]".to_string());
        
        Self {
            uuid: snippet.id.to_string(),
            title: snippet.title,
            content: snippet.content,
            tags: tags_json,
            embedding: snippet.embedding,
        }
    }
}

impl From<DbSnippet> for SnippetWithTags {
    fn from(db_snippet: DbSnippet) -> Self {
        let tags: Vec<String> = serde_json::from_str(&db_snippet.tags).unwrap_or_default();
        
        Self {
            id: Uuid::parse_str(&db_snippet.uuid).unwrap_or_else(|_| Uuid::new_v4()),
            title: db_snippet.title,
            content: db_snippet.content,
            tags,
            embedding: db_snippet.embedding,
            created_at: DateTime::<Utc>::from_utc(db_snippet.created_at, Utc),
            updated_at: DateTime::<Utc>::from_utc(db_snippet.updated_at, Utc),
        }
    }
}

impl From<DbSnippet> for SnippetListItem {
    fn from(snippet: DbSnippet) -> Self {
        Self {
            id: snippet.id,
            uuid: snippet.uuid,
            title: snippet.title,
            tags: snippet.tags,
            updated_at: snippet.updated_at,
        }
    }
}

impl NewDbSnippet {
    /// Create a new snippet with tags
    pub fn new(title: String, content: String, tags: Vec<String>) -> Self {
        let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());
        
        Self {
            uuid: Uuid::new_v4().to_string(),
            title,
            content,
            tags: tags_json,
            embedding: None,
        }
    }
    
    /// Create a new snippet with tags and embedding
    pub fn with_embedding(title: String, content: String, tags: Vec<String>, embedding: Vec<u8>) -> Self {
        let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());
        
        Self {
            uuid: Uuid::new_v4().to_string(),
            title,
            content,
            tags: tags_json,
            embedding: Some(embedding),
        }
    }
}

/// Update data for an existing snippet
#[derive(AsChangeset, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = snippets)]
pub struct UpdateSnippet {
    pub title: Option<String>,
    pub content: Option<String>,
    pub tags: Option<String>, // JSON array stored as string
    pub embedding: Option<Option<Vec<u8>>>, // Option<Option<T>> to handle setting to NULL
    pub updated_at: NaiveDateTime,
}

impl Default for UpdateSnippet {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateSnippet {
    /// Create an update with tags
    pub fn new() -> Self {
        Self {
            title: None,
            content: None,
            tags: None,
            embedding: None,
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
    
    /// Set the title
    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }
    
    /// Set the content
    pub fn with_content(mut self, content: String) -> Self {
        self.content = Some(content);
        self
    }
    
    /// Set the tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());
        self.tags = Some(tags_json);
        self
    }
    
    /// Set the embedding
    pub fn with_embedding(mut self, embedding: Option<Vec<u8>>) -> Self {
        self.embedding = Some(embedding);
        self
    }
}