//! CLI commands module

use clap::{Args, Subcommand};

// Command modules
pub mod add;
pub mod list;
pub mod snippets;
pub mod stash_cmds;
pub mod use_snippet;

// --- Top-level Command Groups ---

#[derive(Args)]
pub struct SnippetCommand {
    #[command(subcommand)]
    pub command: SnippetCommands,
}

#[derive(Args)]
pub struct StashCommand {
    #[command(subcommand)]
    pub command: stash_cmds::StashCommands,
}

// --- Subcommands for Snippets ---

#[derive(Subcommand)]
pub enum SnippetCommands {
    /// Add a new snippet
    Add(add::AddCommand),
    /// List and search snippets
    List(list::ListCommand),
    /// Use a snippet (expand and copy to clipboard)
    Use(use_snippet::UseCommand),
}
