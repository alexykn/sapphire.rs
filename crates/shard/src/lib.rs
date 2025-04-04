// Shard - Package management tool for macOS using Homebrew

// Core modules
pub mod core;
pub mod package;
pub mod brew;
pub mod shard;
pub mod utils;

// CLI handling
pub mod cli;

// Re-export error handling and logging
pub use utils::{
    ShardError, ShardResult, ResultExt,
    log_success, log_warning, log_error, log_step,
    Logger, LogLevel
};

// Re-export common types and functions for convenience
pub use core::manifest;
pub use brew::BrewClient;
pub use shard::{
    apply::{apply, apply_all_enabled_shards},
    diff::diff,
    init::init_shards,
    manager::{disable_shard, enable_shard, grow_shard, shatter_shard}
};

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME"); 