//! # Rustash CLI
//!
//! Command-line interface for the Rustash snippet manager.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use anyhow::Result;
use clap::{Parser, Subcommand};
use rustash_core::{Config, Database, Snippet, SnippetService};
use rustash_utils::config::load_config;

/// Rustash - A modern snippet manager
#[derive(Parser)]
#[command(name = "rustash")]
#[command(version = rustash_core::VERSION)]
#[command(about = "A modern Rust-based snippet manager")]
#[command(long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new snippet
    Add {
        /// Snippet title
        title: String,
        /// Snippet content
        content: String,
        /// Category (optional)
        #[arg(short, long)]
        category: Option<String>,
        /// Tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,
    },
    /// List all snippets
    List {
        /// Category filter
        #[arg(short, long)]
        category: Option<String>,
        /// Tag filter
        #[arg(short, long)]
        tag: Option<String>,
        /// Search query
        #[arg(short, long)]
        search: Option<String>,
    },
    /// Use a snippet (expand and copy to clipboard)
    Use {
        /// Snippet ID or title
        id: String,
        /// Skip interactive placeholder input
        #[arg(short, long)]
        no_interactive: bool,
    },
    /// Delete a snippet
    Delete {
        /// Snippet ID
        id: String,
    },
    /// Edit a snippet
    Edit {
        /// Snippet ID
        id: String,
    },
    /// Show snippet details
    Show {
        /// Snippet ID
        id: String,
    },
    /// Search snippets
    Search {
        /// Search query
        query: String,
        /// Use vector search
        #[arg(short, long)]
        vector: bool,
    },
    /// Initialize database and configuration
    Init {
        /// Force reinitialize
        #[arg(short, long)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Load configuration
    let config = match cli.config {
        Some(path) => load_config(&path)?,
        None => load_config("rustash.toml").unwrap_or_default(),
    };
    
    // Initialize database
    let database = Database::new(&config.database_url)?;
    let service = SnippetService::new(database);
    
    match cli.command {
        Commands::Add { title, content, category, tags } => {
            let snippet = Snippet::new(
                title,
                content,
                category.unwrap_or_else(|| config.default_category.clone()),
                tags.map(|t| t.split(',').map(|s| s.trim().to_string()).collect()).unwrap_or_default(),
            );
            
            let id = service.create_snippet(snippet).await?;
            println!("Created snippet with ID: {}", id);
        }
        Commands::List { category, tag, search } => {
            let snippets = service.list_snippets(category, tag, search).await?;
            
            if snippets.is_empty() {
                println!("No snippets found.");
            } else {
                for snippet in snippets {
                    println!("{}: {} [{}]", snippet.id, snippet.title, snippet.category);
                }
            }
        }
        Commands::Use { id, no_interactive } => {
            let snippet = service.get_snippet_by_id_or_title(&id).await?;
            let expanded = if no_interactive {
                snippet.content
            } else {
                service.expand_placeholders(&snippet.content).await?
            };
            
            service.copy_to_clipboard(&expanded).await?;
            println!("Copied to clipboard: {}", snippet.title);
        }
        Commands::Delete { id } => {
            service.delete_snippet(&id).await?;
            println!("Deleted snippet: {}", id);
        }
        Commands::Edit { id } => {
            let snippet = service.get_snippet_by_id_or_title(&id).await?;
            println!("Edit functionality not yet implemented for: {}", snippet.title);
        }
        Commands::Show { id } => {
            let snippet = service.get_snippet_by_id_or_title(&id).await?;
            println!("Title: {}", snippet.title);
            println!("Category: {}", snippet.category);
            println!("Tags: {:?}", snippet.tags);
            println!("Content:\n{}", snippet.content);
        }
        Commands::Search { query, vector } => {
            let results = if vector {
                service.vector_search(&query, 10).await?
            } else {
                service.text_search(&query, 10).await?
            };
            
            for snippet in results {
                println!("{}: {} [{}]", snippet.id, snippet.title, snippet.category);
            }
        }
        Commands::Init { force } => {
            service.initialize_database(force).await?;
            println!("Database initialized successfully.");
        }
    }
    
    Ok(())
}