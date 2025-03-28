use std::path::PathBuf;
use std::env;
use console::style;
use shellexpand;
use crate::core::manifest::Manifest;
use crate::utils::{
    ShardResult, ResultExt, 
    log_success, log_warning, log_step, log_debug,
    ensure_dir_exists
};

const SHARDS_DIR: &str = "~/.sapphire/shards";
const DISABLED_DIR: &str = "~/.sapphire/disabled";

/// Initialize default system and user shards
pub fn init_shards(force: bool) -> ShardResult<()> {
    log_step("Initializing system and user shards");
    
    let shards_dir = PathBuf::from(shellexpand::tilde(SHARDS_DIR).into_owned());
    let disabled_dir = PathBuf::from(shellexpand::tilde(DISABLED_DIR).into_owned());
    
    // Create directories if they don't exist
    ensure_dir_exists(&shards_dir)
        .with_context(|| format!("Failed to create shards directory: {}", shards_dir.display()))?;
    ensure_dir_exists(&disabled_dir)
        .with_context(|| format!("Failed to create disabled shards directory: {}", disabled_dir.display()))?;
    
    // Get system shard path
    let system_path = PathBuf::from(format!("{}/system.toml", shards_dir.display()));
    
    // Create system shard if it doesn't exist or force overwrite
    if !system_path.exists() || force {
        create_system_shard(&system_path)?;
    } else {
        log_warning("System shard already exists. Use --force to overwrite.");
    }
    
    // Get user shard path
    let user_path = PathBuf::from(format!("{}/user.toml", shards_dir.display()));
    
    // Create user shard if it doesn't exist or force overwrite
    if !user_path.exists() || force {
        let username = get_username()?;
        create_user_shard(&user_path, &username)?;
    } else {
        log_warning("User shard already exists. Use --force to overwrite.");
    }
    
    log_success("Initialization complete!");
    Ok(())
}

/// Get current username
fn get_username() -> ShardResult<String> {
    // Try to get username from environment
    match env::var("USER") {
        Ok(username) if !username.is_empty() => {
            log_debug(&format!("Found username from environment: {}", username));
            Ok(username)
        },
        _ => {
            log_warning("Could not determine username from environment, using 'user' as default");
            Ok("user".to_string())
        }
    }
}

/// Create system shard with default packages
fn create_system_shard(path: &PathBuf) -> ShardResult<()> {
    let mut manifest = Manifest::new();
    
    // Set metadata
    manifest.metadata.name = "system".to_string();
    manifest.metadata.description = "System-level packages".to_string();
    manifest.metadata.protected = true;
    
    // Add some common taps
    manifest.taps.push("homebrew/core".to_string());
    manifest.taps.push("homebrew/cask".to_string());
    
    // Write to file
    manifest.to_file(path.to_str().unwrap_or_default())
        .with_context(|| format!("Failed to create system shard at {}", path.display()))?;
    
    log_success(&format!("Created system shard at {}", style(path.display()).bold()));
    Ok(())
}

/// Create user shard with personal packages
fn create_user_shard(path: &PathBuf, username: &str) -> ShardResult<()> {
    let mut manifest = Manifest::new();
    
    // Set metadata
    manifest.metadata.name = "user".to_string();
    manifest.metadata.description = format!("User packages for {}", username);
    manifest.metadata.owner = username.to_string();
    
    // Write to file
    manifest.to_file(path.to_str().unwrap_or_default())
        .with_context(|| format!("Failed to create user shard at {}", path.display()))?;
    
    log_success(&format!("Created user shard at {}", style(path.display()).bold()));
    Ok(())
}