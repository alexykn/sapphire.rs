use anyhow::{Result, Context};
use console::style;
use std::path::PathBuf;
use std::fs;
use sapphire_core::utils::file_system as fs_utils;
use shellexpand;
use crate::core::manifest::Manifest;
use std::env;
use std::path::Path;

const SHARDS_DIR: &str = "~/.sapphire/shards";
const DISABLED_DIR: &str = "~/.sapphire/disabled";

/// Initialize default system and user shards
pub fn init_shards(force: bool) -> Result<()> {
    println!("{} system and user shards", style("Initializing").bold().green());
    
    let shards_dir = PathBuf::from(shellexpand::tilde(SHARDS_DIR).into_owned());
    let disabled_dir = PathBuf::from(shellexpand::tilde(DISABLED_DIR).into_owned());
    
    // Create directories if they don't exist
    fs_utils::ensure_dir_exists(&shards_dir)?;
    fs_utils::ensure_dir_exists(&disabled_dir)?;
    
    // Get system shard path
    let system_path = PathBuf::from(format!("{}/system.toml", shards_dir.display()));
    
    // Create system shard if it doesn't exist or force overwrite
    if !system_path.exists() || force {
        create_system_shard(&system_path)?;
    }
    
    // Get username
    let username = get_username()?;
    
    // Get user shard path
    let user_path = PathBuf::from(format!("{}/{}_user.toml", shards_dir.display(), username));
    
    // Create user shard if it doesn't exist or force overwrite
    if !user_path.exists() || force {
        create_user_shard(&user_path, &username)?;
    }
    
    Ok(())
}

/// Get the current username
fn get_username() -> Result<String> {
    match env::var("USER") {
        Ok(username) => Ok(username),
        Err(_) => {
            // Fallback to "user" if we can't get the username
            Ok("user".to_string())
        }
    }
}

/// Create a system shard file
fn create_system_shard(path: &PathBuf) -> Result<()> {
    let mut system_manifest = Manifest::new();
    system_manifest.metadata.description = "System-level packages".to_string();
    system_manifest.metadata.protected = true; // System shard is protected
    
    if path.exists() {
        println!("{} Overwriting existing system shard", style("Warning:").bold().yellow());
    }
    
    // Save the system manifest
    system_manifest.to_file(path)
        .context("Failed to create system shard")?;
    
    println!("{} Created system shard: {}", style("✓").bold().green(), style(path.display()).bold());
    Ok(())
}

/// Create a user shard file
fn create_user_shard(path: &PathBuf, username: &str) -> Result<()> {
    let mut user_manifest = Manifest::new();
    user_manifest.metadata.description = format!("User packages for {}", username);
    user_manifest.metadata.protected = true; // User shard is also protected
    
    if path.exists() {
        println!("{} Overwriting existing user shard", style("Warning:").bold().yellow());
    }
    
    // Save the user manifest
    user_manifest.to_file(path)
        .context("Failed to create user shard")?;
    
    println!("{} Created user shard: {}", style("✓").bold().green(), style(path.display()).bold());
    Ok(())
}