//! Data models for Rustash

use crate::memory::MemoryItem;
use crate::schema::snippets;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use diesel::prelude::*;
use diesel::sql_types::{Text, Timestamp};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/// A snippet stored in the database
#[derive(Queryable, Selectable, Serialize, Deserialize, Debug, Clone, PartialEq, QueryableByName)]
#[diesel(table_name = crate::schema::snippets)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DbSnippet {
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
#[diesel(table_name = crate::schema::snippets)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SnippetListItem {
    #[diesel(sql_type = Text)]
    pub uuid: String,
    #[diesel(sql_type = Text)]
    pub title: String,
    #[diesel(sql_type = Text)]
    pub tags: String, // JSON array stored as string
    #[diesel(sql_type = Timestamp)]
    pub updated_at: NaiveDateTime,
}

/// A snippet with parsed tags for easier handling
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SnippetWithTags {
    /// The UUID of the snippet as a string for easy serialization/deserialization
    #[serde(rename = "id")]
    pub uuid: String,
    
    /// The parsed Uuid for internal use
    #[serde(skip)]
    pub id: Uuid,
    
    pub title: String,
    pub content: String,
    pub tags: Vec<String>, // Parsed from JSON
    pub embedding: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SnippetWithTags {
    /// Create a new SnippetWithTags with the given UUID
    pub fn with_uuid(uuid: Uuid, title: String, content: String, tags: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            uuid: uuid.to_string(),
            id: uuid,
            title,
            content,
            tags,
            embedding: None,
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Get the UUID as a Uuid type
    pub fn id(&self) -> Uuid {
        self.id
    }
}

/// The main Snippet struct that implements MemoryItem
#[derive(Queryable, Selectable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = crate::schema::snippets)]
pub struct Snippet {
    pub uuid: String,
    pub title: String,
    pub content: String,
    pub tags: String, // Stored as JSON string in the database
    pub embedding: Option<Vec<u8>>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Snippet {
    /// Get the UUID as a Uuid type
    pub fn id(&self) -> Uuid {
        // This should ideally never fail as we validate UUIDs when creating/updating
        Uuid::parse_str(&self.uuid).unwrap_or_else(|_| {
            // In case of invalid UUID (shouldn't happen), generate a new one
            // This is a fallback and indicates a data consistency issue
            Uuid::new_v4()
        })
    }
    
    /// Create a new Snippet with the given UUID
    pub fn with_uuid(uuid: Uuid, title: String, content: String, tags: Vec<String>) -> Self {
        let now = Utc::now().naive_utc();
        Self {
            uuid: uuid.to_string(),
            title,
            content,
            tags: serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string()),
            embedding: None,
            created_at: now,
            updated_at: now,
        }
    }
}

impl fmt::Display for Snippet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Snippet {{ id: {}, title: {} }}", self.uuid, self.title)
    }
}

// Use the CloneDyn implementation from memory.rs

impl MemoryItem for Snippet {
    fn id(&self) -> Uuid { 
        self.id()
    }
    
    fn item_type(&self) -> &'static str { 
        "snippet" 
    }
    
    fn content(&self) -> &str { 
        &self.content 
    }
    
    fn metadata(&self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        map.insert("title".to_string(), Value::String(self.title.clone()));
        
        // Parse tags from JSON string
        let tags: Vec<String> = serde_json::from_str(&self.tags).unwrap_or_default();
        map.insert("tags".to_string(), Value::Array(tags.into_iter().map(Value::String).collect()));
        
        // Add timestamps
        map.insert("created_at".to_string(), Value::String(self.created_at.to_string()));
        map.insert("updated_at".to_string(), Value::String(self.updated_at.to_string()));
        
        // Add the UUID as a string for easy access
        map.insert("uuid".to_string(), Value::String(self.uuid.clone()));
        
        map
    }
    
    fn created_at(&self) -> DateTime<Utc> {
        Utc.from_utc_datetime(&self.created_at)
    }
    
    fn updated_at(&self) -> DateTime<Utc> {
        Utc.from_utc_datetime(&self.updated_at)
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn clone_dyn(&self) -> Box<dyn MemoryItem> {
        Box::new(self.clone())
    }
}

// Conversion implementations

impl From<DbSnippet> for Snippet {
    fn from(db_snippet: DbSnippet) -> Self {
        Self {
            uuid: db_snippet.uuid,
            title: db_snippet.title,
            content: db_snippet.content,
            tags: db_snippet.tags, // Store tags as JSON string
            embedding: db_snippet.embedding,
            created_at: db_snippet.created_at,
            updated_at: db_snippet.updated_at,
        }
    }
}

impl From<Snippet> for NewDbSnippet {
    fn from(snippet: Snippet) -> Self {
        Self {
            uuid: snippet.uuid,
            title: snippet.title,
            content: snippet.content,
            tags: snippet.tags, // Already in JSON string format
            embedding: snippet.embedding,
        }
    }
}

impl From<DbSnippet> for SnippetWithTags {
    fn from(db_snippet: DbSnippet) -> Self {
        let tags: Vec<String> = serde_json::from_str(&db_snippet.tags).unwrap_or_default();
        let uuid = Uuid::parse_str(&db_snippet.uuid).unwrap_or_else(|_| Uuid::new_v4());
        
        Self {
            uuid: db_snippet.uuid,
            id: uuid,
            title: db_snippet.title,
            content: db_snippet.content,
            tags,
            embedding: db_snippet.embedding,
            created_at: DateTime::<Utc>::from_naive_utc_and_offset(db_snippet.created_at, Utc),
            updated_at: DateTime::<Utc>::from_naive_utc_and_offset(db_snippet.updated_at, Utc),
        }
    }
}

impl From<Snippet> for SnippetWithTags {
    fn from(snippet: Snippet) -> Self {
        let tags: Vec<String> = serde_json::from_str(&snippet.tags).unwrap_or_default();
        let uuid = Uuid::parse_str(&snippet.uuid).unwrap_or_else(|_| Uuid::new_v4());
        
        Self {
            uuid: snippet.uuid,
            id: uuid,
            title: snippet.title,
            content: snippet.content,
            tags,
            embedding: snippet.embedding,
            created_at: DateTime::<Utc>::from_naive_utc_and_offset(snippet.created_at, Utc),
            updated_at: DateTime::<Utc>::from_naive_utc_and_offset(snippet.updated_at, Utc),
        }
    }
}

impl From<DbSnippet> for SnippetListItem {
    fn from(snippet: DbSnippet) -> Self {
        Self {
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