use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::fs;
use console::style;
use crate::parser::Fragment;
use sapphire_core::utils::file_system as fs_utils;

/// Check for differences in configuration fragments
pub fn diff<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    
    let files = if path.is_dir() {
        // Get all .toml files in the directory
        let entries = fs::read_dir(path)
            .with_context(|| format!("Failed to read directory: {}", path.display()))?;
            
        let mut yaml_files = Vec::new();
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map(|ext| ext == "yaml" || ext == "yml").unwrap_or(false) {
                yaml_files.push(path);
            }
        }
        
        yaml_files
    } else {
        // Single file
        vec![path.to_path_buf()]
    };
    
    if files.is_empty() {
        tracing::warn!("No fragment files found at: {}", path.display());
        return Ok(());
    }
    
    let mut checked = 0;
    let mut with_diffs = 0;
    
    for file in &files {
        match check_fragment_diff(file) {
            Ok(has_diffs) => {
                checked += 1;
                if has_diffs {
                    with_diffs += 1;
                }
            }
            Err(err) => {
                tracing::error!("Failed to check fragment {}: {}", file.display(), err);
            }
        }
    }
    
    tracing::info!("Checked {} fragments, {} with differences", checked, with_diffs);
    
    Ok(())
}

/// Check for differences in a single fragment file
fn check_fragment_diff(path: &Path) -> Result<bool> {
    if !fs_utils::path_exists(path) {
        anyhow::bail!("Fragment file does not exist: {}", path.display());
    }
    
    let fragment = Fragment::from_file(path)?;
    
    tracing::info!("Checking fragment: {}", path.display());
    tracing::info!("Fragment type: {:?}, Description: {}", fragment.fragment_type, fragment.description);
    
    // TODO: Implement diff checking based on fragment type
    
    // Placeholder for diff detection
    let has_diffs = false;
    
    if !has_diffs {
        tracing::info!("No differences found in {}", path.display());
    }
    
    Ok(has_diffs)
}