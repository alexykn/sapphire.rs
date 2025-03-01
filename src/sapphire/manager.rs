use anyhow::{Context, Result};
use std::fs;

/// Get the current Sapphire version
pub fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Get the sapphire directory path (~/.sapphire)
pub fn get_sapphire_dir() -> Result<std::path::PathBuf> {
    let home_dir = dirs::home_dir()
        .context("Unable to determine home directory")?;
    
    Ok(home_dir.join(".sapphire"))
}

/// Get the configuration directory path (same as sapphire directory)
pub fn get_config_dir() -> Result<std::path::PathBuf> {
    get_sapphire_dir()
}

/// Get the data directory path (same as sapphire directory)
pub fn get_data_dir() -> Result<std::path::PathBuf> {
    get_sapphire_dir()
}

/// Check if Sapphire is properly set up
pub fn check_installation() -> Result<bool> {
    let config_dir = get_config_dir()?;
    let config_file = config_dir.join("config.toml");
    
    if !config_file.exists() {
        return Ok(false);
    }
    
    let data_dir = get_data_dir()?;
    
    // Check if required directories exist
    let required_dirs = [
        "fragments",
        "scripts",
        "manifests",
    ];
    
    for dir in required_dirs.iter() {
        if !data_dir.join(dir).exists() {
            return Ok(false);
        }
    }
    
    Ok(true)
}

/// Load configuration
pub fn load_config() -> Result<toml::Table> {
    let config_dir = get_config_dir()?;
    let config_file = config_dir.join("config.toml");
    
    if !config_file.exists() {
        anyhow::bail!("Configuration file not found: {}", config_file.display());
    }
    
    let config_content = fs::read_to_string(&config_file)
        .context(format!("Failed to read configuration file: {}", config_file.display()))?;
    
    let config: toml::Table = toml::from_str(&config_content)
        .context("Failed to parse configuration file")?;
    
    Ok(config)
}

/// Get a specific configuration value
pub fn get_config_value(key: &str) -> Result<Option<String>> {
    let config = load_config()?;
    
    // Handle nested keys (e.g. "paths.fragments")
    let parts: Vec<&str> = key.split('.').collect();
    
    if parts.len() == 1 {
        if let Some(value) = config.get(key) {
            return Ok(Some(value.to_string()));
        }
    } else if parts.len() == 2 {
        if let Some(section) = config.get(parts[0]) {
            if let Some(table) = section.as_table() {
                if let Some(value) = table.get(parts[1]) {
                    return Ok(Some(value.to_string()));
                }
            }
        }
    }
    
    Ok(None)
}

/// Set a configuration value
pub fn set_config_value(key: &str, value: &str) -> Result<()> {
    let mut config = load_config()?;
    
    // Handle nested keys (e.g. "paths.fragments")
    let parts: Vec<&str> = key.split('.').collect();
    
    if parts.len() == 1 {
        config.insert(key.to_string(), toml::Value::String(value.to_string()));
    } else if parts.len() == 2 {
        if let Some(section) = config.get_mut(parts[0]) {
            if let Some(table) = section.as_table_mut() {
                table.insert(parts[1].to_string(), toml::Value::String(value.to_string()));
            } else {
                anyhow::bail!("Configuration key '{}' is not a table", parts[0]);
            }
        } else {
            // Create the section if it doesn't exist
            let mut new_section = toml::Table::new();
            new_section.insert(parts[1].to_string(), toml::Value::String(value.to_string()));
            config.insert(parts[0].to_string(), toml::Value::Table(new_section));
        }
    } else {
        anyhow::bail!("Invalid configuration key format: {}", key);
    }
    
    // Write the updated configuration back to the file
    let config_dir = get_config_dir()?;
    let config_file = config_dir.join("config.toml");
    
    let config_content = toml::to_string(&config)
        .context("Failed to serialize configuration")?;
    
    fs::write(&config_file, config_content)
        .context(format!("Failed to write configuration file: {}", config_file.display()))?;
    
    Ok(())
}