// Fragment binary entry point
use anyhow::Result;
use fragment;

fn main() -> Result<()> {
    fragment::cli::run()
} 