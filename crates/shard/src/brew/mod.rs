//! Homebrew package manager integration.
//!
//! This module provides a Rust interface to the Homebrew package manager CLI.
//! It supports operations such as installing, uninstalling, searching, and
//! managing packages with a focus on security, proper error handling, and
//! a clean API.
//!
//! The module is organized into specialized components:
//! - `client`: Primary user-facing API and coordination
//! - `core`: Low-level command execution
//! - `installer`: Package installation and management
//! - `search`: Package search and information
//! - `validate`: Input validation and security
//!
//! # Security
//!
//! All user inputs are validated to prevent command injection vulnerabilities.
//! The validation module provides the security primitives used throughout.

pub mod client;
pub mod core;
pub mod installer;
pub mod search;
pub mod validate;

// Re-export common types and functions
pub use client::BrewClient;
pub use core::BrewCore;
pub use installer::BrewInstaller;
pub use search::BrewSearcher;
pub use search::{FormulaInfo, CaskInfo};

// Convenience function to get a brew client
pub fn get_client() -> client::BrewClient {
    client::BrewClient::new()
} 