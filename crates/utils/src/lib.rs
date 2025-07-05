//! # Rustash Utilities
//!
//! Utility functions and helpers for the Rustash project.

#![deny(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

pub mod config;
pub mod time;
pub mod validation;

pub use config::*;
pub use time::*;
pub use validation::*;

/// Result type for utility operations
pub type Result<T> = anyhow::Result<T>;