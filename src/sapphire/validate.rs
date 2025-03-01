use std::path::Path;
use anyhow::{Context, Result};
use crate::core::config::SapphireConfig;
use crate::utils::fs;

/// Validate a Sapphire configuration
pub fn validate<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    
    // Ensure the directory exists
    if !fs::path_exists(path) {
        anyhow::bail!("Path does not exist: {}", path.display());
    }
    
    // Determine the configuration file path
    let config_path = if path.is_dir() {
        path.join("sapphire.yml")
    } else {
        path.to_path_buf()
    };
    
    // Check if the config file exists
    if !fs::path_exists(&config_path) {
        anyhow::bail!("Configuration file does not exist: {}", config_path.display());
    }
    
    // Load and validate the configuration
    let _config = SapphireConfig::from_file(&config_path)
        .context(format!("Failed to load configuration from {}", config_path.display()))?;
    
    // Additional validation logic can be added here
    
    tracing::info!("Configuration at {} is valid", config_path.display());
    
    Ok(())
}