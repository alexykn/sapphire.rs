// Sapphire Core - shared components for all Sapphire tools

pub mod logging;
pub mod utils;

// Re-export common traits and types
pub use logging::error::{SapphireError, SapphireResult}; 