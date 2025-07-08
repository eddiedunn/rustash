//! Utility functions for CLI

use anyhow::Result;
use arboard::Clipboard;
use console::{style, Term};
use rustash_core::models::SnippetWithTags;
use std::io::Write;

/// Copy text to clipboard
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard =
        Clipboard::new().map_err(|e| anyhow::anyhow!("Failed to access clipboard: {}", e))?;

    clipboard
        .set_text(text)
        .map_err(|e| anyhow::anyhow!("Failed to copy to clipboard: {}", e))?;

    Ok(())
}

/// Format and display a list of snippets
pub fn format_snippet_list(snippets: &[SnippetWithTags], format: &str) -> Result<()> {
    match format {
        "table" => format_table(snippets),
        "compact" => format_compact(snippets),
        "detailed" => format_detailed(snippets),
        _ => anyhow::bail!(
            "Unknown format '{}'. Use: table, compact, detailed, json, ids",
            format
        ),
    }
}

fn format_table(snippets: &[SnippetWithTags]) -> Result<()> {
    if snippets.is_empty() {
        return Ok(());
    }

    let mut term = Term::stdout();

    // Header
    writeln!(
        term,
        "{} {} {} {}",
        style("ID").bold().cyan(),
        style("Title").bold().cyan(),
        style("Tags").bold().cyan(),
        style("Updated").bold().cyan()
    )?;

    writeln!(term, "{}", "─".repeat(80))?;

    // Rows
    for snippet in snippets {
        let tags_str = if snippet.tags.is_empty() {
            style("").dim().to_string()
        } else {
            style(snippet.tags.join(", ")).yellow().to_string()
        };

        let title = if snippet.title.len() > 40 {
            format!("{}...", &snippet.title[..37])
        } else {
            snippet.title.clone()
        };

        let updated = snippet.updated_at.format("%Y-%m-%d %H:%M").to_string();

        writeln!(
            term,
            "{:<4} {:<43} {:<20} {}",
            &snippet.uuid[..8],
            title,
            tags_str,
            style(updated).dim()
        )?;
    }

    Ok(())
}

fn format_compact(snippets: &[SnippetWithTags]) -> Result<()> {
    let mut term = Term::stdout();

    for snippet in snippets {
        let tags_str = if snippet.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", snippet.tags.join(", "))
        };

        writeln!(
            term,
            "{}: {}{}",
            style(&snippet.uuid).green().bold(),
            snippet.title,
            style(tags_str).yellow()
        )?;
    }

    Ok(())
}

fn format_detailed(snippets: &[SnippetWithTags]) -> Result<()> {
    let mut term = Term::stdout();

    for (i, snippet) in snippets.iter().enumerate() {
        if i > 0 {
            writeln!(term, "{}", "─".repeat(80))?;
        }

        writeln!(term, "{}: {}", style("ID").bold(), style(&snippet.uuid).green())?;
        writeln!(term, "{}: {}", style("Title").bold(), snippet.title)?;

        if !snippet.tags.is_empty() {
            writeln!(
                term,
                "{}: {}",
                style("Tags").bold(),
                style(snippet.tags.join(", ")).yellow()
            )?;
        }

        writeln!(
            term,
            "{}: {}",
            style("Created").bold(),
            snippet.created_at.format("%Y-%m-%d %H:%M:%S").to_string()
        )?;
        writeln!(
            term,
            "{}: {}",
            style("Updated").bold(),
            snippet.updated_at.format("%Y-%m-%d %H:%M:%S").to_string()
        )?;

        writeln!(term, "{}:", style("Content").bold())?;

        // Display content with proper indentation
        let content_lines: Vec<&str> = snippet.content.lines().collect();
        let preview_lines = if content_lines.len() > 10 {
            &content_lines[..10]
        } else {
            &content_lines
        };

        for line in preview_lines {
            writeln!(term, "  {}", line)?;
        }

        if content_lines.len() > 10 {
            writeln!(term, "  {}", style("... (truncated)").dim())?;
        }
    }

    Ok(())
}
