use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{Level, debug};
use tracing_subscriber::{fmt, EnvFilter};
use crate::{apply, diff, init};
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
            .add_directive(format!("fragment={}", level).parse().unwrap());
        
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
#[command(author, version, about = "Fragment configuration tool", long_about = None)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Apply configuration fragment
    Apply {
        /// Path to fragment file
        #[arg(default_value = "~/.sapphire/fragments/user")]
        path: String,
        
        /// Dry run without making changes
        #[arg(short, long)]
        dry_run: bool,
    },
    
    /// Check fragment for changes
    Diff {
        /// Path to fragment file
        #[arg(default_value = "~/.sapphire/fragments/user")]
        path: String,
    },
    
    /// Create new fragment from template
    Init {
        /// Fragment type
        #[arg(long, default_value = "dotfiles")]
        fragment_type: String,
        
        /// Path to create fragment file
        #[arg(default_value = "~/.sapphire/fragments/user")]
        path: String,
        
        /// Override existing fragment
        #[arg(short, long)]
        force: bool,
    },
    
    /// Set system configuration
    Config {
        /// Configuration domain (e.g., "com.apple.finder")
        domain: String,
        
        /// Key to set
        key: String,
        
        /// Value to set (omit to read current value)
        value: Option<String>,
        
        /// Value type (string, bool, int, float, array)
        #[arg(short, long, default_value = "string")]
        r#type: String,
    },
    
    /// Run a task defined in a fragment
    Run {
        /// Task name
        task: String,
        
        /// Fragment path (default is user fragment)
        #[arg(short, long, default_value = "~/.sapphire/fragments/user")]
        fragment: String,
    },
    
    /// List all available tasks in a fragment
    Tasks {
        /// Fragment path
        #[arg(default_value = "~/.sapphire/fragments/user")]
        fragment: String,
    },
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logger
    init_logging(cli.verbose);
    
    match cli.command {
        Commands::Apply { path, dry_run } => {
            apply::apply(&path, dry_run)
        },
        Commands::Diff { path } => {
            diff::diff(&path)
        },
        Commands::Init { fragment_type, path, force: _ } => {
            init::init(&fragment_type, &path)
        },
        Commands::Config { domain, key, value: _, r#type: _ } => {
            // TODO: Implement config functionality
            println!("Setting config for domain {}, key {}", domain, key);
            Ok(())
        },
        Commands::Run { task, fragment } => {
            // TODO: Implement task runner
            println!("Running task {} from fragment {}", task, fragment);
            Ok(())
        },
        Commands::Tasks { fragment } => {
            // TODO: Implement task listing
            println!("Listing tasks from fragment {}", fragment);
            Ok(())
        },
    }
} 