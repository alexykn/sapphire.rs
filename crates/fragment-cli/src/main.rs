use anyhow::Result;
use clap::{Parser, Subcommand};
use sapphire_core::utils::logger;
use sapphire_core::fragment;

#[derive(Debug, Parser)]
#[command(author, version, about = "Fragment system configuration tool", long_about = None)]
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logger with verbosity level
    logger::init_logger(cli.verbose)?;
    
    match cli.command {
        Commands::Apply { path, dry_run } => {
            fragment::apply::apply(&path, dry_run)
        },
        Commands::Diff { path } => {
            fragment::diff::diff(&path)
        },
        Commands::Init { fragment_type, path, force } => {
            fragment::init::init(&fragment_type, &path)
        },
        Commands::Config { domain, key, value, r#type } => {
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