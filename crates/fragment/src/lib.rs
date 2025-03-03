// Fragment - Configuration management tool for macOS

// Configuration management functionality
pub mod apply;
pub mod diff;
pub mod engine;
pub mod init;
pub mod parser;

// CLI handling
pub mod cli;

// Utilities
pub mod utils;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME"); 