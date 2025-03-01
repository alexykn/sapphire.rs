use anyhow::Result;
use clap::{Parser, Subcommand};
use crate::utils::logger;
use crate::sapphire;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Initialize Sapphire system (first-time setup)
    Setup {
        /// Installation mode (local or managed)
        #[arg(long, default_value = "local")]
        mode: String,
    },

    /// Manage Sapphire application itself
    App {
        #[command(subcommand)]
        action: AppCommands,
    },
    
    /// Manage packages and shards (package collections)
    Shard {
        #[command(subcommand)]
        action: ShardCommands,
    },
    
    /// Manage system configuration (shortcut for 'fragment')
    Fragment {
        #[command(subcommand)]
        action: FragmentCommands,
    },
}

#[derive(Debug, Subcommand)]
enum AppCommands {
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

#[derive(Debug, Subcommand)]
enum ShardCommands {
    /// Apply a shard to install/remove packages
    Apply {
        /// Path to shard file or "all" to apply all enabled shards
        #[arg(default_value = "~/.sapphire/shards/user.toml")]
        shard: String,
        
        /// Dry run without making changes
        #[arg(short, long)]
        dry_run: bool,
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

#[derive(Debug, Subcommand)]
enum FragmentCommands {
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
    },
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logger
    logger::init_logger(cli.verbose)?;

    match &cli.command {
        Some(Commands::Setup { mode }) => {
            sapphire::setup::initialize(mode)?;
        }
        Some(Commands::App { action }) => {
            match action {
                AppCommands::Update => {
                    println!("Updating Sapphire...");
                    // TODO: Implement update logic
                }
                AppCommands::Status => {
                    println!("Sapphire Status:");
                    // TODO: Implement status check
                }
                AppCommands::Config { key, value } => {
                    if let Some(k) = key {
                        if let Some(v) = value {
                            println!("Setting config {}={}", k, v);
                            // TODO: Implement config setting
                        } else {
                            println!("Getting config value for {}", k);
                            // TODO: Implement config getting
                        }
                    } else {
                        println!("Current configuration:");
                        // TODO: List all config
                    }
                }
            }
        }
        Some(Commands::Shard { action }) => {
            match action {
                ShardCommands::Apply { shard, dry_run } => {
                    if shard == "all" {
                        crate::shard::apply::apply_all_enabled_shards(*dry_run, false)?;
                    } else {
                        crate::shard::apply::apply(shard, *dry_run, false)?;
                    }
                }
                ShardCommands::Convert { force } => {
                    crate::shard::migrate::convert_yaml_to_toml(*force)?;
                }
                ShardCommands::Diff { shard } => {
                    crate::shard::diff::diff(shard)?;
                }
                ShardCommands::Init { force } => {
                    crate::shard::init::init_shards(*force)?;
                }
                ShardCommands::Search { query, r#type, deep } => {
                    crate::shard::search::search(&query, &r#type, *deep)?;
                }
                ShardCommands::Add { packages, brew, cask, shard, dry_run } => {
                    crate::shard::package::add_packages(&packages, *brew, *cask, &shard, *dry_run)?;
                }
                ShardCommands::Del { packages, brew, cask, shard, dry_run } => {
                    crate::shard::package::remove_packages(&packages, *brew, *cask, &shard, *dry_run)?;
                }
                ShardCommands::Migrate { system_nix, user_nix, name, non_interactive, dry_run } => {
                    crate::shard::migrate::migrate_from_nix(
                        system_nix.as_deref(), 
                        user_nix.as_deref(), 
                        name.as_deref(), 
                        *non_interactive,
                        *dry_run
                    )?;
                }
                ShardCommands::Grow { name, description } => {
                    crate::shard::manage::grow_shard(&name, description.as_deref())?;
                }
                ShardCommands::Shatter { name, force } => {
                    crate::shard::manage::shatter_shard(&name, *force)?;
                }
                ShardCommands::Disable { name } => {
                    crate::shard::manage::disable_shard(&name)?;
                }
                ShardCommands::Enable { name } => {
                    crate::shard::manage::enable_shard(&name)?;
                }
            }
        }
        Some(Commands::Fragment { action }) => {
            match action {
                FragmentCommands::Apply { path, dry_run } => {
                    crate::fragment::apply::apply(path, *dry_run)?;
                }
                FragmentCommands::Diff { path } => {
                    crate::fragment::diff::diff(path)?;
                }
                FragmentCommands::Init { fragment_type, path } => {
                    crate::fragment::init::init(fragment_type, path)?;
                }
            }
        }
        None => {
            println!("No command specified. Use --help for usage information.");
        }
    }

    Ok(())
}