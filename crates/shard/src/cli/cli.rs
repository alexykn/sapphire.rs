use clap::{Parser, Subcommand};
use crate::utils::ShardResult;
use crate::utils::observability::{Logger, LogLevel};

use crate::{
    brew::search,
    package::operations as package,
    shard::{
        apply, diff, init,
        manager as manage,
    }
};

#[derive(Debug, Parser)]
#[command(author, version = crate::VERSION, about = "Shard package management tool", long_about = None)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Apply a shard to install/remove packages
    Apply {
        /// Path to shard file or "all" to apply all enabled shards
        #[arg(default_value = "~/.sapphire/shards/user.toml")]
        shard: String,
        
        /// Skip cleanup after applying
        #[arg(long)]
        skip_cleanup: bool,
    },
    
    /// Check what would change if a shard was applied
    Diff {
        /// Path to shard file or "all" to check all enabled shards
        #[arg(default_value = "~/.sapphire/shards/user.toml")]
        shard: String,
    },
    
    /// Initialize default system and user shards
    Init {
        /// Force overwrite if shards already exist
        #[arg(short, long)]
        force: bool,
    },
    
    /// Create a new named shard in the shards directory
    Grow {
        /// Name of the new shard (will be created as ~/.sapphire/shards/<n>.toml)
        name: String,
        
        /// Description of the shard's purpose
        #[arg(short, long)]
        description: Option<String>,
    },
    
    /// Delete a shard permanently
    Shatter {
        /// Name of the shard to delete (from ~/.sapphire/shards/<n>.toml)
        name: String,
        
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    
    /// Disable a shard without deleting it (moves to disabled directory)
    Disable {
        /// Name of the shard to disable
        name: String,
    },
    
    /// Enable a previously disabled shard
    Enable {
        /// Name of the shard to enable
        name: String,
    },
    
    /// Search for packages
    Search {
        /// Search query
        query: String,
        
        /// Search type (brew, cask, any)
        #[arg(short, long, default_value = "any")]
        r#type: String,
        
        /// Show more details
        #[arg(short, long)]
        deep: bool,
    },
    
    /// Add packages to a shard and install them
    Add {
        /// Packages to add
        #[arg(required = true)]
        packages: Vec<String>,
        
        /// Force brew formulas (vs casks)
        #[arg(long)]
        formula: bool,
        
        /// Force casks (vs brew formulas)
        #[arg(long)]
        cask: bool,
        
        /// Specify which shard to modify (use 'user' for user shard, 'system' for system shard, or a custom shard name)
        #[arg(short = 's', long = "shard", default_value = "user")]
        shard: String,
        
        /// Dry run without making changes
        #[arg(long)]
        dry_run: bool,

        /// Immediately install *only* the added packages without a full apply
        #[arg(long, conflicts_with = "apply")]
        exec: bool,

        /// Immediately run 'apply all' after adding packages to the shard
        #[arg(long, conflicts_with = "exec")]
        apply: bool,
    },
    
    /// Remove packages from a shard
    Del {
        /// Packages to remove
        #[arg(required = true)]
        packages: Vec<String>,
        
        /// Force brew formulas (vs casks)
        #[arg(long)]
        formula: bool,
        
        /// Force casks (vs brew formulas)
        #[arg(long)]
        cask: bool,
        
        /// Specify which shard to modify (use 'user' for user shard, 'system' for system shard, or a custom shard name, or 'all' to search all shards)
        #[arg(short = 's', long = "shard", default_value = "all")]
        shard: String,
        
        /// Dry run without making changes
        #[arg(long)]
        dry_run: bool,

        /// Immediately uninstall *only* the removed packages without a full apply
        #[arg(long, conflicts_with = "apply")]
        exec: bool,

        /// Immediately run 'apply all' after removing packages from the shard(s)
        #[arg(long, conflicts_with = "exec")]
        apply: bool,
    },
}

pub fn run() -> ShardResult<()> {
    let cli = Cli::parse();
    
    // Set log level based on verbosity
    let log_level = if cli.verbose { LogLevel::Debug } else { LogLevel::Info }; // Default to Info
    Logger::init(log_level);
    
    match cli.command {
        Commands::Apply { shard, skip_cleanup } => {
            if shard.to_lowercase() == "all" {
                apply::apply_all_enabled_shards(skip_cleanup)
            } else {
                apply::apply(&shard, skip_cleanup)
            }
        },
        Commands::Diff { shard } => {
            diff::diff(&shard)
        },
        Commands::Init { force } => {
            init::init_shards(force)
        },
        Commands::Grow { name, description } => {
            manage::grow_shard(&name, description.as_deref())
        },
        Commands::Shatter { name, force } => {
            manage::shatter_shard(&name, force)
        },
        Commands::Disable { name } => {
            manage::disable_shard(&name)
        },
        Commands::Enable { name } => {
            manage::enable_shard(&name)
        },
        Commands::Search { query, r#type, deep } => {
            search::search(&query, &r#type, deep)
        },
        Commands::Add { packages, formula, cask, shard, dry_run, exec, apply } => {
            package::add_packages(&packages, formula, cask, &shard, dry_run, exec, apply)
        },
        Commands::Del { packages, formula, cask, shard, dry_run, exec, apply } => {
            package::remove_packages(&packages, formula, cask, &shard, dry_run, exec, apply)
        },
    }
} 