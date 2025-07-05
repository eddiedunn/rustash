//! Utility functions for CLI

use anyhow::Result;
use arboard::Clipboard;
use rustash_core::SnippetWithTags;
use console::{style, Term};
use std::io::Write;

/// Copy text to clipboard
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| anyhow::anyhow!("Failed to access clipboard: {}", e))?;
    
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
        _ => anyhow::bail!("Unknown format '{}'. Use: table, compact, detailed, json, ids", format),
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
        let id_str = snippet.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string());
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
            style(id_str).green(),
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
        let id_str = snippet.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string());
        let tags_str = if snippet.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", snippet.tags.join(", "))
        };
        
        writeln!(
            term,
            "{}: {}{}",
            style(id_str).green().bold(),
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
        
        let id_str = snippet.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string());
        
        writeln!(term, "{}: {}", style("ID").bold(), style(id_str).green())?;
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
            snippet.created_at.format("%Y-%m-%d %H:%M:%S")
        )?;
        writeln!(
            term,
            "{}: {}",
            style("Updated").bold(),
            snippet.updated_at.format("%Y-%m-%d %H:%M:%S")
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

/// Get terminal width for formatting
pub fn get_terminal_width() -> usize {
    Term::stdout().size().1 as usize
}

/// Truncate text to fit within a given width
pub fn truncate_text(text: &str, max_width: usize) -> String {
    if text.len() <= max_width {
        text.to_string()
    } else if max_width <= 3 {
        "...".to_string()
    } else {
        format!("{}...", &text[..max_width - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("hello", 10), "hello");
        assert_eq!(truncate_text("hello world", 5), "he...");
        assert_eq!(truncate_text("hello", 3), "...");
        assert_eq!(truncate_text("hello", 2), "...");
    }
}