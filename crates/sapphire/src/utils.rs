use std::path::Path;
use std::fs;
use anyhow::{Result, Context, anyhow};

// Result type for Sapphire operations
pub type SapphireResult<T> = Result<T>;

// File system utilities
pub fn path_exists(path: &Path) -> bool {
    path.exists()
}

pub fn file_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

pub fn ensure_dir_exists(path: &Path) -> SapphireResult<()> {
    if !path.exists() {
        fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory: {}", path.display()))?;
    } else if !path.is_dir() {
        return Err(anyhow!("Path exists but is not a directory: {}", path.display()));
    }
    Ok(())
}

pub fn read_file(path: &Path) -> SapphireResult<String> {
    fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))
}

pub fn write_file(path: &Path, content: &str) -> SapphireResult<()> {
    if let Some(parent) = path.parent() {
        ensure_dir_exists(parent)?;
    }
    fs::write(path, content)
        .with_context(|| format!("Failed to write file: {}", path.display()))
} 