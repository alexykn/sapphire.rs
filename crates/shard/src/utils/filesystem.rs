use std::fs;
use std::path::{Path, PathBuf};
use std::io;

use crate::utils::{ShardResult, ShardError, ResultExt};
use shellexpand;

/// Ensures a directory exists, creating it if necessary
pub fn ensure_dir_exists(path: &Path) -> ShardResult<()> {
    if !path.exists() {
        fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory: {}", path.display()))?;
    } else if !path.is_dir() {
        return Err(ShardError::Filesystem { 
            path: path.to_path_buf(), 
            source: io::Error::new(io::ErrorKind::AlreadyExists, "Path exists but is not a directory") 
        });
    }
    Ok(())
}

/// Ensures a parent directory exists for a file path
pub fn ensure_parent_dir_exists(file_path: &Path) -> ShardResult<()> {
    if let Some(parent) = file_path.parent() {
        ensure_dir_exists(parent)?;
    }
    Ok(())
}

/// Checks if a file exists
pub fn file_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

/// Checks if a path exists (file or directory)
pub fn path_exists(path: &Path) -> bool {
    path.exists()
}

/// Copies a file, ensuring the target directory exists
pub fn copy_file(source: &Path, target: &Path) -> ShardResult<()> {
    ensure_parent_dir_exists(target)?;
    fs::copy(source, target)
        .with_context(|| format!("Failed to copy file from {} to {}", 
            source.display(), target.display()))?;
    Ok(())
}

/// Renames a file or directory, ensuring the target directory exists
pub fn rename_path(source: &Path, target: &Path) -> ShardResult<()> {
    if let Some(parent) = target.parent() {
        ensure_dir_exists(parent)?;
    }
    fs::rename(source, target)
        .with_context(|| format!("Failed to rename from {} to {}", 
            source.display(), target.display()))?;
    Ok(())
}

/// Removes a file if it exists
pub fn remove_file(path: &Path) -> ShardResult<()> {
    if path.exists() && path.is_file() {
        fs::remove_file(path)
            .with_context(|| format!("Failed to remove file: {}", path.display()))?;
    }
    Ok(())
}

/// Creates backup of a file with .bak extension
pub fn backup_file(path: &Path) -> ShardResult<Option<PathBuf>> {
    if !path.exists() || !path.is_file() {
        return Ok(None);
    }
    
    let backup_path = PathBuf::from(format!("{}.bak", path.display()));
    copy_file(path, &backup_path)?;
    Ok(Some(backup_path))
}

/// Resolve a manifest name or path to a full path
/// Handles special shard names like "user", "system", or any custom shard name
/// Returns a full path to the manifest file
pub fn resolve_manifest_path(manifest_target: &str) -> ShardResult<String> {
    // If it looks like a path, just expand tilde
    if manifest_target.contains('/') || manifest_target.ends_with(".toml") {
        Ok(shellexpand::tilde(manifest_target).to_string())
    } else {
        // Assume it's a shard name (validate it)
        crate::brew::validate::validate_package_name(manifest_target)
            .with_context(|| format!("Invalid shard name: {}", manifest_target))?;
        let shards_dir = shellexpand::tilde("~/.sapphire/shards").to_string();
        Ok(format!("{}/{}.toml", shards_dir, manifest_target))
    }
} 