use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Get the sapphire configuration directory (~/.sapphire)
pub fn get_config_dir() -> Result<PathBuf> {
    let home_dir = dirs::home_dir()
        .context("Unable to determine home directory")?;
    
    let config_dir = home_dir.join(".sapphire");
    
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)
            .context("Failed to create configuration directory")?;
    }
    
    Ok(config_dir)
}

/// Ensure a directory exists, creating it if necessary
pub fn ensure_dir_exists<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    
    if !path.exists() {
        std::fs::create_dir_all(path)
            .context(format!("Failed to create directory: {}", path.display()))?;
    } else if !path.is_dir() {
        anyhow::bail!("Path exists but is not a directory: {}", path.display());
    }
    
    Ok(())
}

/// Check if a path exists
pub fn path_exists<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().exists()
}

/// Resolve a path, expanding ~ to the home directory
pub fn resolve_path<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    let path_str = path.as_ref().to_string_lossy();
    
    if path_str.starts_with("~/") {
        let home = dirs::home_dir()
            .context("Unable to determine home directory")?;
        
        let rel_path = &path_str[2..];
        Ok(home.join(rel_path))
    } else {
        Ok(path.as_ref().to_path_buf())
    }
}