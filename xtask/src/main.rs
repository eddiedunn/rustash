//! # Rustash Build Tasks
//!
//! Custom build automation tasks for the Rustash project.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build automation tasks for Rustash")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all tests
    Test,
    /// Run linting
    Lint,
    /// Format code
    Format,
    /// Check code coverage
    Coverage,
    /// Build all crates
    Build,
    /// Run security audit
    Audit,
    /// Generate documentation
    Doc,
    /// Release preparation
    Release,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Test => {
            println!("Running tests...");
            std::process::Command::new("cargo")
                .args(["nextest", "run"])
                .status()?;
        }
        Commands::Lint => {
            println!("Running linting...");
            std::process::Command::new("cargo")
                .args(["clippy", "--all-targets", "--all-features", "--", "-D", "warnings"])
                .status()?;
        }
        Commands::Format => {
            println!("Formatting code...");
            std::process::Command::new("cargo")
                .args(["fmt", "--check"])
                .status()?;
        }
        Commands::Coverage => {
            println!("Checking coverage...");
            std::process::Command::new("cargo")
                .args(["tarpaulin", "--out", "Html", "--fail-under", "80"])
                .status()?;
        }
        Commands::Build => {
            println!("Building all crates...");
            std::process::Command::new("cargo")
                .args(["build", "--all-features"])
                .status()?;
        }
        Commands::Audit => {
            println!("Running security audit...");
            std::process::Command::new("cargo")
                .args(["audit"])
                .status()?;
        }
        Commands::Doc => {
            println!("Generating documentation...");
            std::process::Command::new("cargo")
                .args(["doc", "--document-private-items"])
                .status()?;
        }
        Commands::Release => {
            println!("Preparing release...");
            // Run all checks
            std::process::Command::new("cargo")
                .args(["fmt", "--check"])
                .status()?;
            std::process::Command::new("cargo")
                .args(["clippy", "--all-targets", "--all-features", "--", "-D", "warnings"])
                .status()?;
            std::process::Command::new("cargo")
                .args(["nextest", "run"])
                .status()?;
            std::process::Command::new("cargo")
                .args(["audit"])
                .status()?;
            println!("Release preparation complete!");
        }
    }
    
    Ok(())
}