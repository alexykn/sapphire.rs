use anyhow::{Context, Result};
use std::path::Path;
use std::fs;
use crate::parser::Fragment;
use sapphire_core::utils::file_system as fs_utils;

/// Apply configuration fragments
pub fn apply<P: AsRef<Path>>(path: P, dry_run: bool) -> Result<()> {
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
    
    let mut applied = 0;
    let mut failed = 0;
    
    for file in &files {
        match apply_fragment(file, dry_run) {
            Ok(_) => {
                applied += 1;
            }
            Err(err) => {
                tracing::error!("Failed to apply fragment {}: {}", file.display(), err);
                failed += 1;
            }
        }
    }
    
    tracing::info!("Applied {} fragments, {} failed", applied, failed);
    
    if failed > 0 {
        anyhow::bail!("Failed to apply {} fragments", failed);
    }
    
    Ok(())
}

/// Apply a single fragment file
fn apply_fragment(path: &Path, dry_run: bool) -> Result<()> {
    if !fs_utils::path_exists(path) {
        anyhow::bail!("Fragment file does not exist: {}", path.display());
    }
    
    let fragment = Fragment::from_file(path)?;
    
    tracing::info!("Applying fragment: {}", path.display());
    tracing::info!("Fragment type: {:?}, Description: {}", fragment.fragment_type, fragment.description);
    
    if dry_run {
        tracing::info!("Dry run - no changes will be made");
    }
    
    // TODO: Implement fragment application based on type
    
    Ok(())
}