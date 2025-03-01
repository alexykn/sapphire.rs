use anyhow::{Context, Result};
use dialoguer::Confirm;
use console::style;
use std::path::Path;
use std::process::Command;
use shellexpand;

use crate::shard::manifest::{Manifest, Formula, Cask, PackageState};
use crate::utils::fs;

/// Package type enum
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PackageType {
    Brew,
    Cask,
}

/// Package availability result
#[derive(Debug)]
pub struct PackageAvailability {
    pub name: String,
    pub available_as_brew: bool,
    pub available_as_cask: bool,
}

/// Add packages to manifest and install them
pub fn add_packages(packages: &[String], force_brew: bool, force_cask: bool, manifest_path: &str, dry_run: bool) -> Result<()> {
    // Resolve the manifest path
    let manifest_path = resolve_manifest_path(manifest_path)?;
    
    // First, load or create the manifest
    let mut manifest = if fs::path_exists(&manifest_path) {
        Manifest::from_file(&manifest_path)?
    } else {
        println!("Manifest file not found. Creating a new one.");
        let manifest = Manifest::new();
        if !dry_run {
            // Create parent directories if needed
            if let Some(parent) = Path::new(&manifest_path).parent() {
                fs::ensure_dir_exists(parent)?;
            }
        }
        manifest
    };
    
    // Check if this is a protected manifest
    if manifest.metadata.protected {
        println!("{} Cannot modify protected shard: {}", 
            style("Error:").bold().red(), 
            style(manifest_path).bold());
        return Ok(());
    }
    
    // Check if this is a system shard using ShardManager
    if let Ok(is_protected) = crate::shard::manage::is_protected_shard(
        Path::new(&manifest_path).file_stem().unwrap_or_default().to_string_lossy().as_ref()
    ) {
        if is_protected {
            println!("{} Cannot modify protected shard: {}", 
                style("Error:").bold().red(), 
                style(manifest_path).bold());
            return Ok(());
        }
    }
    
    // Track if we've added any new packages
    let mut added_any_packages = false;
    
    // Process each package
    for package_name in packages {
        // Check if package is already in the manifest
        let already_in_formulas = manifest.formulas.iter().any(|f| f.name == *package_name);
        let already_in_formulae = manifest.formulae.contains(package_name);
        let already_in_casks = manifest.casks.iter().any(|c| c.name == *package_name);
        let already_in_brews = manifest.brews.contains(package_name);
        
        // If package is already present in any form, skip it
        if already_in_formulas || already_in_formulae || already_in_casks || already_in_brews {
            // Determine if it's a formula or cask
            let package_type = if already_in_formulas || already_in_formulae {
                "formula"
            } else {
                "cask"
            };
            println!("{} is already in the manifest as a {}, skipping", 
                style(package_name).bold(), 
                style(package_type).bold());
            continue;
        }
        
        match process_package(package_name, force_brew, force_cask, dry_run)? {
            Some((package_type, state)) => {
                added_any_packages = true;
                
                // First, remove any existing entries to avoid duplicates
                // We'll do this silently without printing messages
                if !dry_run {
                    // Remove from formulas list
                    manifest.formulas.retain(|f| f.name != *package_name);
                    // Remove from formulae list
                    manifest.formulae.retain(|f| f != package_name);
                    // Remove from casks list
                    manifest.casks.retain(|c| c.name != *package_name);
                    // Remove from brews list
                    manifest.brews.retain(|b| b != package_name);
                }
                
                // Add to manifest based on type
                match package_type {
                    PackageType::Brew => {
                        // Check if already in complex formulas list after removal
                        let existing_brew = manifest.formulas.iter().position(|f| f.name == *package_name);
                        if let Some(index) = existing_brew {
                            println!("Updating existing formula: {}", package_name);
                            manifest.formulas[index].state = state;
                        } else {
                            println!("Adding formula: {}", package_name);
                            manifest.formulas.push(Formula {
                                name: package_name.clone(),
                                version: "latest".to_string(),
                                options: Vec::new(),
                                state,
                            });
                        }
                        
                        // Also add to simplified formulae list if not already there
                        if !manifest.formulae.contains(package_name) {
                            manifest.formulae.push(package_name.clone());
                        }
                    },
                    PackageType::Cask => {
                        // Check if already in complex casks list after removal
                        let existing_cask = manifest.casks.iter().position(|c| c.name == *package_name);
                        if let Some(index) = existing_cask {
                            println!("Updating existing cask: {}", package_name);
                            manifest.casks[index].state = state;
                        } else {
                            println!("Adding cask: {}", package_name);
                            manifest.casks.push(Cask {
                                name: package_name.clone(),
                                version: "latest".to_string(),
                                options: Vec::new(),
                                state,
                            });
                        }
                        
                        // Also add to simplified brews list if not already there
                        if !manifest.brews.contains(package_name) {
                            manifest.brews.push(package_name.clone());
                        }
                    },
                }
            },
            None => {
                println!("Skipping {}", package_name);
                continue;
            }
        }
    }
    
    // Save manifest and apply changes only if we added any packages
    if !dry_run && added_any_packages {
        manifest.to_file(&manifest_path)?;
        println!("Manifest updated: {}", manifest_path);
        
        // Apply the manifest with a quiet option for "already installed" messages
        println!("Applying changes to remove package(s)...");
        
        // Custom handling for package removal - apply with a parameter to avoid 
        // showing "already installed" messages during package removal
        let manifest = crate::shard::manifest::Manifest::from_file(std::path::Path::new(&manifest_path))
            .with_context(|| format!("Failed to load manifest file: {}", manifest_path))?;
        crate::shard::apply::apply_internal(&manifest, false, false)?;
    } else if !dry_run && !added_any_packages {
        println!("No changes made to manifest - all packages were already present");
    } else {
        println!("Dry run - no changes made to manifest");
    }
    Ok(())
}

/// Remove packages from manifest
pub fn remove_packages(packages: &[String], force_brew: bool, force_cask: bool, manifest_path: &str, dry_run: bool) -> Result<()> {
    // If manifest_path is not "all", resolve the specific manifest path
    if manifest_path != "all" {
        let specific_manifest_path = resolve_manifest_path(manifest_path)?;
        
        // First, load the manifest
        if !fs::path_exists(&specific_manifest_path) {
            anyhow::bail!("Manifest file not found: {}", specific_manifest_path);
        }
        
        let _ = remove_packages_from_manifest(packages, force_brew, force_cask, &specific_manifest_path, dry_run)?;
        return Ok(());
    }
    
    // If we're here, we need to search through all manifests
    println!("Searching for packages in all manifests...");
    
    // Get path to shards directory
    let shards_dir = shellexpand::tilde("~/.sapphire/shards").to_string();
    
    // Check if shards directory exists
    if !fs::path_exists(&shards_dir) {
        println!("No shards directory found. Nothing to remove.");
        return Ok(());
    }
    
    // Read all toml files in the directory
    let entries = std::fs::read_dir(&shards_dir)
        .with_context(|| format!("Failed to read shards directory: {}", shards_dir))?;
    
    // Collect paths to process
    let mut shard_paths = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        // Skip directories and non-toml files
        if path.is_dir() || path.extension().map_or(true, |ext| ext != "toml") {
            continue;
        }
        
        // Skip system manifest
        if path.file_name().map_or(false, |name| name == "system.toml") {
            println!("Skipping system manifest (protected)");
            continue;
        }
        
        shard_paths.push(path);
    }
    
    // Sort by filename to ensure consistent order
    shard_paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    
    if shard_paths.is_empty() {
        println!("No shards found in directory. Nothing to remove from.");
        return Ok(());
    }
    
    // Track if we found any packages
    let mut found_any = false;
    
    // Process each shard
    for shard_path in shard_paths {
        let shard_path_str = shard_path.to_string_lossy().to_string();
        let shard_name = shard_path.file_stem().unwrap().to_string_lossy();
        
        // Try to remove packages from this shard
        match remove_packages_from_manifest(packages, force_brew, force_cask, &shard_path_str, dry_run) {
            Ok(found) => {
                if found {
                    println!("Removed packages from shard: {}", style(&shard_name).bold());
                    found_any = true;
                }
            },
            Err(e) => {
                println!("Error processing shard {}: {}", shard_name, e);
            }
        }
    }
    
    if !found_any {
        println!("No packages found to remove in any shard.");
    }
    
    Ok(())
}

/// Remove packages from a specific manifest file
fn remove_packages_from_manifest(packages: &[String], force_brew: bool, force_cask: bool, manifest_path: &str, dry_run: bool) -> Result<bool> {
    let mut manifest = Manifest::from_file(manifest_path)?;
    let mut found_packages = false;
    
    // Check if this is a protected manifest
    if manifest.metadata.protected {
        println!("{} Cannot modify protected shard: {}", 
            style("Error:").bold().red(), 
            style(manifest_path).bold());
        return Ok(false);
    }
    
    // Check if this is a system shard using ShardManager
    if let Ok(is_protected) = crate::shard::manage::is_protected_shard(
        Path::new(manifest_path).file_stem().unwrap_or_default().to_string_lossy().as_ref()
    ) {
        if is_protected {
            println!("{} Cannot modify protected shard: {}", 
                style("Error:").bold().red(), 
                style(manifest_path).bold());
            return Ok(false);
        }
    }
    
    // Process each package
    for package_name in packages {
        let mut found = false;
        let mut removed_as_formula = false;
        let mut removed_as_cask = false;
        
        // Check formulas if not forcing cask only
        if !force_cask || force_brew {
            // Collect all indices in the complex formulas structure
            let formula_indices: Vec<usize> = manifest.formulas
                .iter()
                .enumerate()
                .filter(|(_, f)| f.name == *package_name)
                .map(|(i, _)| i)
                .collect();
            
            // Remove all occurrences
            if !formula_indices.is_empty() {
                found = true;
                found_packages = true;
                removed_as_formula = true;
                
                if !dry_run {
                    // Remove in reverse order to avoid index shifting
                    for idx in formula_indices.into_iter().rev() {
                        manifest.formulas.remove(idx);
                    }
                }
            }
            
            // Also check in the simplified formulae list
            let formulae_indices: Vec<usize> = manifest.formulae
                .iter()
                .enumerate()
                .filter(|(_, f)| f == &package_name)
                .map(|(i, _)| i)
                .collect();
            
            if !formulae_indices.is_empty() {
                found = true;
                found_packages = true;
                removed_as_formula = true;
                
                if !dry_run {
                    // Remove in reverse order to avoid index shifting
                    for idx in formulae_indices.into_iter().rev() {
                        manifest.formulae.remove(idx);
                    }
                }
            }
        }
        
        // Check casks if not forcing brew only
        if !force_brew || force_cask {
            // Collect all indices in the complex casks structure
            let cask_indices: Vec<usize> = manifest.casks
                .iter()
                .enumerate()
                .filter(|(_, c)| c.name == *package_name)
                .map(|(i, _)| i)
                .collect();
            
            // Remove all occurrences
            if !cask_indices.is_empty() {
                found = true;
                found_packages = true;
                removed_as_cask = true;
                
                if !dry_run {
                    // Remove in reverse order to avoid index shifting
                    for idx in cask_indices.into_iter().rev() {
                        manifest.casks.remove(idx);
                    }
                }
            }
            
            // Also check in the simplified brews list
            let brew_indices: Vec<usize> = manifest.brews
                .iter()
                .enumerate()
                .filter(|(_, b)| b == &package_name)
                .map(|(i, _)| i)
                .collect();
            
            if !brew_indices.is_empty() {
                found = true;
                found_packages = true;
                removed_as_cask = true;
                
                if !dry_run {
                    // Remove in reverse order to avoid index shifting
                    for idx in brew_indices.into_iter().rev() {
                        manifest.brews.remove(idx);
                    }
                }
            }
        }
        
        // Print removal messages once for each package type
        if removed_as_formula {
            println!("Removing formula {}", package_name);
        }
        if removed_as_cask {
            println!("Removing cask {}", package_name);
        }
        
        if !found {
            println!("Package {} not found in manifest", package_name);
        }
    }
    
    if !found_packages {
        return Ok(false);
    }
    
    // Save manifest
    if !dry_run {
        manifest.to_file(manifest_path)?;
        println!("Manifest updated: {}", manifest_path);
        
        // Apply the manifest with a quiet option for "already installed" messages
        println!("Applying changes to remove package(s)...");
        
        // Custom handling for package removal - apply with a parameter to avoid 
        // showing "already installed" messages during package removal
        let manifest = crate::shard::manifest::Manifest::from_file(std::path::Path::new(manifest_path))
            .with_context(|| format!("Failed to load manifest file: {}", manifest_path))?;
        crate::shard::apply::apply_internal(&manifest, false, false)?;
    } else {
        println!("Dry run - no changes made to manifest");
    }
    
    Ok(found_packages)
}

/// Resolve a manifest path, handling special cases like "user" or "system"
fn resolve_manifest_path(manifest_path: &str) -> Result<String> {
    // If it's a full path, just expand it
    if manifest_path.contains('/') || manifest_path.ends_with(".toml") {
        return Ok(shellexpand::tilde(manifest_path).to_string());
    }
    
    // Handle special cases
    let shards_dir = shellexpand::tilde("~/.sapphire/shards").to_string();
    
    match manifest_path {
        "system" => Ok(format!("{}/system.toml", shards_dir)),
        "user" => {
            // Get username for user shard
            let username = match std::env::var("USER") {
                Ok(username) => username,
                Err(_) => "user".to_string(),
            };
            
            Ok(format!("{}/{}_user.toml", shards_dir, username))
        },
        _ => {
            // Assume it's a custom shard name
            Ok(format!("{}/{}.toml", shards_dir, manifest_path))
        }
    }
}

/// Process a package and determine its type
fn process_package(package_name: &str, force_brew: bool, force_cask: bool, dry_run: bool) -> Result<Option<(PackageType, PackageState)>> {
    // Check package availability
    let availability = check_package_availability(package_name)?;
    
    // Determine package type based on availability and user preferences
    let package_type = if force_brew {
        if availability.available_as_brew {
            Some(PackageType::Brew)
        } else if availability.available_as_cask {
            if !dry_run {
                let confirm = Confirm::new()
                    .with_prompt(format!("Package '{}' not found as brew formula but is available as cask. Install as cask?", package_name))
                    .default(true)
                    .interact()?;
                
                if confirm {
                    Some(PackageType::Cask)
                } else {
                    println!("Installation aborted.");
                    None
                }
            } else {
                println!("Would prompt for confirmation to install as cask instead (dry run)");
                Some(PackageType::Cask)
            }
        } else {
            println!("Package '{}' not found as brew formula or cask", package_name);
            None
        }
    } else if force_cask {
        if availability.available_as_cask {
            Some(PackageType::Cask)
        } else if availability.available_as_brew {
            if !dry_run {
                let confirm = Confirm::new()
                    .with_prompt(format!("Package '{}' not found as cask but is available as brew formula. Install as brew?", package_name))
                    .default(true)
                    .interact()?;
                
                if confirm {
                    Some(PackageType::Brew)
                } else {
                    println!("Installation aborted.");
                    None
                }
            } else {
                println!("Would prompt for confirmation to install as brew instead (dry run)");
                Some(PackageType::Brew)
            }
        } else {
            println!("Package '{}' not found as cask or brew formula", package_name);
            None
        }
    } else {
        // Auto mode - prefer cask, fallback to brew
        if availability.available_as_cask {
            println!("Found {} as cask", style(package_name).bold());
            Some(PackageType::Cask)
        } else if availability.available_as_brew {
            println!("Found {} as brew formula", style(package_name).bold());
            Some(PackageType::Brew)
        } else {
            println!("Package '{}' not found as cask or brew formula", package_name);
            None
        }
    };
    
    match package_type {
        Some(pkg_type) => Ok(Some((pkg_type, PackageState::Latest))),
        None => Ok(None),
    }
}

/// Check if a package is available as brew and/or cask
fn check_package_availability(package_name: &str) -> Result<PackageAvailability> {
    // Check as brew formula
    let brew_output = Command::new("brew")
        .args(["info", "--formula", package_name])
        .output()
        .context(format!("Failed to check brew formula: {}", package_name))?;
    
    let available_as_brew = brew_output.status.success();
    
    // Check as cask
    let cask_output = Command::new("brew")
        .args(["info", "--cask", package_name])
        .output()
        .context(format!("Failed to check cask: {}", package_name))?;
    
    let available_as_cask = cask_output.status.success();
    
    Ok(PackageAvailability {
        name: package_name.to_string(),
        available_as_brew,
        available_as_cask,
    })
}