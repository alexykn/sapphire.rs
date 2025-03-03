// Shard - Package management tool for macOS using Homebrew

// Package management functionality
pub mod apply;
pub mod brew_client;
pub mod diff;
pub mod init;
pub mod manage;
pub mod manifest;
pub mod migrate;
pub mod package;
pub mod package_processor;
pub mod search;
pub mod utils;

// CLI handling
pub mod cli;

// Re-export core functionality
pub use sapphire_core::{SapphireError, SapphireResult};

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME"); 