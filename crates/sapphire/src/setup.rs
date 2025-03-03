use std::path::Path;
use anyhow::{Context, Result};
use sapphire_core::utils::file_system as fs;

/// Initialize Sapphire environment for first-time setup
pub fn initialize(mode: &str) -> Result<()> {
    // Validate mode
    let mode = match mode {
        "local" => "local",
        "managed" => "managed",
        _ => {
            anyhow::bail!("Invalid mode: {}. Must be 'local' or 'managed'", mode);
        }
    };

    tracing::info!("Initializing Sapphire in {} mode", mode);
    
    // Get the user's home directory
    let home_dir = dirs::home_dir()
        .context("Unable to determine home directory")?;
    
    // Create the .sapphire directory in home
    let base_dir = home_dir.join(".sapphire");
    
    // Create directory structure
    create_directory_structure(&base_dir)?;
    
    // Create initial configuration (config is also in .sapphire)
    let config_dir = base_dir.clone();
    create_initial_config(&config_dir, mode)?;
    
    println!("Sapphire initialized successfully in {} mode", mode);
    println!("Sapphire directory: {}", base_dir.display());
    
    Ok(())
}

fn create_directory_structure(base_dir: &Path) -> Result<()> {
    // Create main directories
    let dirs = [
        "fragments/system",
        "fragments/user",
        "scripts",
        "manifests",
        "dotfiles",
    ];
    
    for dir in dirs.iter() {
        let dir_path = base_dir.join(dir);
        fs::ensure_dir_exists(&dir_path)
            .context(format!("Failed to create directory: {}", dir_path.display()))?;
        
        tracing::debug!("Created directory: {}", dir_path.display());
    }
    
    Ok(())
}

fn create_initial_config(config_dir: &Path, mode: &str) -> Result<()> {
    // Create config directory if it doesn't exist
    fs::ensure_dir_exists(config_dir)
        .context(format!("Failed to create config directory: {}", config_dir.display()))?;
    
    // Create initial config.toml 
    let config_path = config_dir.join("config.toml");
    
    if fs::path_exists(&config_path) {
        tracing::warn!("Configuration file already exists: {}", config_path.display());
        return Ok(());
    }
    
    let config_content = format!(r#"# Sapphire Configuration
version = "0.1.0"
mode = "{}"

[paths]
fragments = "~/.sapphire/fragments"
scripts = "~/.sapphire/scripts"
manifests = "~/.sapphire/manifests"
dotfiles = "~/.sapphire/dotfiles"

[features]
package_management = true
configuration_management = true
"#, mode);
    
    std::fs::write(&config_path, config_content)
        .context(format!("Failed to write configuration file: {}", config_path.display()))?;
    
    tracing::debug!("Created configuration file: {}", config_path.display());
    
    Ok(())
}