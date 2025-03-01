use anyhow::Result;
use tracing::Level;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize the logger
pub fn init_logger(verbose: bool) -> Result<()> {
    let filter_level = if verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(EnvFilter::from_default_env().add_directive(filter_level.into()))
        .init();

    Ok(())
}