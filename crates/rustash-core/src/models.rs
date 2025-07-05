//! Data models for Rustash

use crate::schema::snippets;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

/// A snippet stored in the database
#[derive(Queryable, Selectable, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[diesel(table_name = snippets)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Snippet {
    pub id: Option<i32>,
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
pub struct NewSnippet {
    pub title: String,
    pub content: String,
    pub tags: String, // JSON array stored as string
    pub embedding: Option<Vec<u8>>,
}

/// A snippet with parsed tags for easier handling
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SnippetWithTags {
    pub id: Option<i32>,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>, // Parsed from JSON
    pub embedding: Option<Vec<u8>>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl From<Snippet> for SnippetWithTags {
    fn from(snippet: Snippet) -> Self {
        let tags: Vec<String> = serde_json::from_str(&snippet.tags).unwrap_or_default();
        
        Self {
            id: snippet.id,
            title: snippet.title,
            content: snippet.content,
            tags,
            embedding: snippet.embedding,
            created_at: snippet.created_at,
            updated_at: snippet.updated_at,
        }
    }
}

impl NewSnippet {
    /// Create a new snippet with tags
    pub fn new(title: String, content: String, tags: Vec<String>) -> Self {
        let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());
        
        Self {
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