use anyhow::Result;
use clap::{Parser, Subcommand};
use sapphire_core::utils::logger;
use sapphire_core::shard;

#[derive(Debug, Parser)]
#[command(author, version, about = "Shard package management tool", long_about = None)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Apply a shard to install/remove packages
    Apply {
        /// Path to shard file or "all" to apply all enabled shards
        #[arg(default_value = "~/.sapphire/shards/user.toml")]
        shard: String,
        
        /// Dry run without making changes
        #[arg(short, long)]
        dry_run: bool,
        
        /// Skip cleanup after applying
        #[arg(long)]
        skip_cleanup: bool,
    },
    
    /// Convert YAML shards to TOML format
    Convert {
        /// Force conversion even if TOML files already exist
        #[arg(short, long)]
        force: bool,
    },
    
    /// Check what would change if a shard was applied
    Diff {
        /// Path to shard file
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
    
    /// Import packages from Nix configuration to a new shard
    Migrate {
        /// Path to system-level Nix configuration file
        #[arg(short = 's', long = "system_apps")]
        system_nix: Option<String>,
        
        /// Path to user-level Nix configuration file
        #[arg(short = 'u', long = "user_apps")]
        user_nix: Option<String>,
        
        /// Custom name for the generated shard
        #[arg(short = 'n', long = "name")]
        name: Option<String>,
        
        /// Skip interactive package suggestions
        #[arg(short = 'i', long = "non-interactive")]
        non_interactive: bool,
        
        /// Dry run without making changes
        #[arg(short, long)]
        dry_run: bool,
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
        #[arg(short, long)]
        brew: bool,
        
        /// Force casks (vs brew formulas)
        #[arg(short, long)]
        cask: bool,
        
        /// Specify which shard to modify (use 'user' for user shard, 'system' for system shard, or a custom shard name)
        #[arg(short = 's', long = "shard", default_value = "user")]
        shard: String,
        
        /// Dry run without making changes
        #[arg(short, long)]
        dry_run: bool,
    },
    
    /// Remove packages from a shard
    Del {
        /// Packages to remove
        #[arg(required = true)]
        packages: Vec<String>,
        
        /// Force brew formulas (vs casks)
        #[arg(short, long)]
        brew: bool,
        
        /// Force casks (vs brew formulas)
        #[arg(short, long)]
        cask: bool,
        
        /// Specify which shard to modify (use 'user' for user shard, 'system' for system shard, or a custom shard name, or 'all' to search all shards)
        #[arg(short = 's', long = "shard", default_value = "all")]
        shard: String,
        
        /// Dry run without making changes
        #[arg(short, long)]
        dry_run: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logger with verbosity level
    logger::init_logger(cli.verbose)?;
    
    match cli.command {
        Commands::Apply { shard, dry_run, skip_cleanup } => {
            if shard.to_lowercase() == "all" {
                shard::apply::apply_all_enabled_shards(dry_run, skip_cleanup)
            } else {
                shard::apply::apply(&shard, dry_run, skip_cleanup)
            }
        },
        Commands::Convert { force } => {
            shard::migrate::convert_yaml_to_toml(force)
        },
        Commands::Diff { shard } => {
            shard::diff::diff(&shard)
        },
        Commands::Init { force } => {
            shard::init::init_shards(force)
        },
        Commands::Grow { name, description } => {
            shard::manage::grow_shard(&name, description.as_deref())
        },
        Commands::Shatter { name, force } => {
            shard::manage::shatter_shard(&name, force)
        },
        Commands::Disable { name } => {
            shard::manage::disable_shard(&name)
        },
        Commands::Enable { name } => {
            shard::manage::enable_shard(&name)
        },
        Commands::Migrate { system_nix, user_nix, name, non_interactive, dry_run } => {
            shard::migrate::migrate_from_nix(
                system_nix.as_deref(), 
                user_nix.as_deref(), 
                name.as_deref(), 
                non_interactive,
                dry_run
            )
        },
        Commands::Search { query, r#type, deep } => {
            shard::search::search(&query, &r#type, deep)
        },
        Commands::Add { packages, brew, cask, shard, dry_run } => {
            shard::package::add_packages(&packages, brew, cask, &shard, dry_run)
        },
        Commands::Del { packages, brew, cask, shard, dry_run } => {
            shard::package::remove_packages(&packages, brew, cask, &shard, dry_run)
        },
    }
} 