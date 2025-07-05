//! # Rustash Desktop
//!
//! Desktop GUI application for Rustash snippet manager using Tauri.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use anyhow::Result;

#[cfg(feature = "desktop")]
use tauri::Manager;

#[cfg(feature = "desktop")]
fn main() -> Result<()> {
    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_window("main").unwrap();
            window.set_title("Rustash - Snippet Manager")?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    
    Ok(())
}

#[cfg(not(feature = "desktop"))]
fn main() -> Result<()> {
    eprintln!("Desktop feature not enabled. Please build with --features desktop");
    std::process::exit(1);
}