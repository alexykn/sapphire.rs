// Sapphire - System management tool for macOS

// System management functionality
pub mod bootstrap;
pub mod manager;
pub mod setup;

// CLI handling
pub mod cli;

// Utilities
pub mod utils;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME"); 