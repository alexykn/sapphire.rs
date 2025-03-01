use anyhow::{Result, Context};
use console::style;
use std::path::PathBuf;
use crate::utils::fs;
use shellexpand;
use crate::shard::manifest::Manifest;
use std::env;

const SHARDS_DIR: &str = "~/.sapphire/shards";

/// Initialize default system and user shards
pub fn init_shards(force: bool) -> Result<()> {
    println!("{} system and user shards", style("Initializing").bold().green());
    
    // Ensure the shards directory exists
    let shards_dir = shellexpand::tilde(SHARDS_DIR).to_string();
    fs::ensure_dir_exists(&shards_dir)?;
    
    // Get system shard path
    let system_path = PathBuf::from(format!("{}/system.toml", shards_dir));
    
    // Get user shard path with username
    let username = get_username()?;
    let user_path = PathBuf::from(format!("{}/{}_user.toml", shards_dir, username));
    
    // Check if both shards already exist
    let system_exists = system_path.exists();
    let user_exists = user_path.exists();
    
    if system_exists && user_exists && !force {
        println!("{} Both system and user shards already exist", style("Note:").bold().yellow());
        println!("Use 'shard grow <name>' to create additional custom shards");
        return Ok(());
    }
    
    // Create system shard if it doesn't exist (or force is used)
    if !system_exists || force {
        let mut system_manifest = Manifest::new();
        system_manifest.metadata.description = "System-level packages".to_string();
        system_manifest.metadata.protected = true; // System shard is protected
        
        if system_exists && force {
            println!("{} Overwriting existing system shard", style("Warning:").bold().yellow());
        }
        
        // Save the system manifest
        system_manifest.to_file(&system_path)
            .context("Failed to create system shard")?;
        
        println!("{} Created system shard: {}", style("✓").bold().green(), style(system_path.display()).bold());
    } else {
        println!("{} System shard already exists (use --force to overwrite)", style("Note:").bold().yellow());
    }
    
    // Create user shard if it doesn't exist (or force is used)
    if !user_exists || force {
        let mut user_manifest = Manifest::new();
        user_manifest.metadata.description = format!("User packages for {}", username);
        user_manifest.metadata.protected = true; // User shard is also protected
        
        if user_exists && force {
            println!("{} Overwriting existing user shard", style("Warning:").bold().yellow());
        }
        
        // Save the user manifest
        user_manifest.to_file(&user_path)
            .context("Failed to create user shard")?;
        
        println!("{} Created user shard: {}", style("✓").bold().green(), style(user_path.display()).bold());
    } else {
        println!("{} User shard already exists (use --force to overwrite)", style("Note:").bold().yellow());
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