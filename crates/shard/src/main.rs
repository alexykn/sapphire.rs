// Shard binary entry point
use anyhow::Result;
use shard;

fn main() -> Result<()> {
    shard::cli::run()
} 