// Shard binary entry point
use shard::{ShardResult, Logger, LogLevel};

fn main() -> ShardResult<()> {
    // Initialize logging with warn level
    Logger::init(LogLevel::Warn);
    
    // Run the CLI
    shard::cli::run()
} 