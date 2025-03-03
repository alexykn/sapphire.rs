use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{Level, debug};
use tracing_subscriber::{fmt, EnvFilter};
use crate::setup;
use std::sync::Once;

// Static to ensure we only initialize logging once
static INIT_LOGGER: Once = Once::new();

// Initialize logging with the specified verbosity level
fn init_logging(verbose: bool) {
    // Only initialize once
    INIT_LOGGER.call_once(|| {
        let level = if verbose { Level::DEBUG } else { Level::INFO };
        
        // Create a custom filter
        let filter = EnvFilter::from_default_env()
            .add_directive(format!("sapphire={}", level).parse().unwrap());
        
        // Initialize the tracing subscriber
        if let Err(e) = fmt::Subscriber::builder()
            .with_env_filter(filter)
            .with_target(false)
            .with_ansi(true)
            .try_init() {
            eprintln!("Warning: Could not initialize logging: {}", e);
        } else {
            debug!("Logging initialized at level: {}", level);
        }
    });
}

#[derive(Debug, Parser)]
#[command(author, version, about = "Sapphire system management tool", long_about = None)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Initialize Sapphire system (first-time setup)
    Setup {
        /// Installation mode (local or managed)
        #[arg(long, default_value = "local")]
        mode: String,
    },

    /// Update Sapphire application
    Update,
    
    /// Show status of Sapphire components
    Status,
    
    /// Configure Sapphire settings
    Config {
        /// Key to configure
        key: Option<String>,
        
        /// Value to set
        value: Option<String>,
    },
}

/// Run the sapphire CLI
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logger
    init_logging(cli.verbose);
    
    match cli.command {
        Commands::Setup { mode } => {
            setup::initialize(&mode)
        },
        Commands::Update => {
            println!("Updating Sapphire...");
            // TODO: Implement update logic
            Ok(())
        },
        Commands::Status => {
            println!("Sapphire Status:");
            // TODO: Implement status check
            Ok(())
        },
        Commands::Config { key, value } => {
            if let Some(k) = key {
                if let Some(v) = value {
                    println!("Setting config {}={}", k, v);
                    // TODO: Implement config setting
                    Ok(())
                } else {
                    println!("Getting config value for {}", k);
                    // TODO: Implement config getting
                    Ok(())
                }
            } else {
                println!("Current configuration:");
                // TODO: List all config
                Ok(())
            }
        }
    }
} 