// Shard binary entry point
use shard::{ShardResult, log_success};

fn main() -> ShardResult<()> {
    // Initialize logging with default level (INFO)
    shard::init_logging(None);
    
    log_success("Shard initialized successfully");
    
    // Run the CLI
    shard::cli::run()
} 