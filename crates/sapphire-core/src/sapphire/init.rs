use std::path::Path;
use anyhow::{Context, Result};
use crate::core::config::SapphireConfig;
use crate::utils::fs;

/// Initialize a new Sapphire configuration
pub fn initialize<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    
    // Ensure the directory exists
    fs::ensure_dir_exists(path)?;
    
    // Create the configuration file path
    let config_path = path.join("sapphire.yml");
    
    // Check if the config file already exists
    if fs::path_exists(&config_path) {
        anyhow::bail!("Configuration file already exists: {}", config_path.display());
    }
    
    // Create a default configuration
    let config = SapphireConfig::new();
    
    // Save the configuration
    config.to_file(&config_path)
        .context(format!("Failed to write configuration to {}", config_path.display()))?;
    
    tracing::info!("Initialized new Sapphire configuration at {}", config_path.display());
    
    Ok(())
}