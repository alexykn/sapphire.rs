use crate::utils::ShardResult;
use std::path::PathBuf;
use crate::utils::filesystem as fs_utils;
use crate::brew::validate as validation;
use crate::core::manifest::Manifest;
use crate::shard::{apply, manager as shard_manager};
use crate::package::processor::PackageType;
use crate::brew::get_client;
use crate::brew::search::PackageAvailability;
use std::collections::HashMap;
use crate::utils::{ShardError, ResultExt, log_step, log_warning, log_error, log_debug, log_success};
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackageTypeWrapper {
    Formula,
    Cask,
}

impl From<PackageType> for PackageTypeWrapper {
    fn from(pt: PackageType) -> Self {
        match pt {
            PackageType::Formula => PackageTypeWrapper::Formula,
            PackageType::Cask => PackageTypeWrapper::Cask,
        }
    }
}

/// Add packages to manifest and potentially install/apply
pub fn add_packages(
    packages: &[String],
    force_formula: bool, // Renamed from force_brew
    force_cask: bool,
    manifest_name: &str, // Use name instead of path initially
    dry_run: bool,
    exec: bool,          // New flag
    apply_all: bool,     // New flag (renamed from apply)
) -> ShardResult<()> {
    log_step(&format!("Adding packages to shard '{}': {}", manifest_name, packages.join(", ")));
    if dry_run { log_debug("Dry run enabled"); }
    if exec { log_debug("Exec flag enabled: will install added packages immediately"); }
    if apply_all { log_debug("Apply flag enabled: will run 'apply all' after adding"); }

    let brew_client = get_client();

    // Validate all package names first
    for package in packages {
        validation::validate_package_name(package)
            .with_context(|| format!("Invalid package name: {}", package))?;
    }

    // Resolve the manifest path (handles "user", "system", names, or full paths)
    let manifest_path = fs_utils::resolve_manifest_path(manifest_name)?;
    let manifest_path_obj = PathBuf::from(&manifest_path);

    // Load or create the manifest
    let mut manifest = match Manifest::from_file(&manifest_path_obj) {
         Ok(m) => m,
         Err(_) if !fs_utils::path_exists(&manifest_path_obj) => {
              log_warning(&format!("Manifest '{}' not found. Creating new one.", manifest_path));
              if !dry_run {
                  // Ensure parent dir exists before creating
                  if let Some(parent) = manifest_path_obj.parent() {
                      fs_utils::ensure_dir_exists(parent)?;
                  }
              }
              // Create a basic manifest, set name from path
              let mut new_manifest = Manifest::new();
              new_manifest.metadata.name = manifest_path_obj
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
              new_manifest
         }
         Err(e) => return Err(e).with_context(|| format!("Failed to load manifest: {}", manifest_path)),
    };

    // Check protection
    let shard_name_for_check = manifest_path_obj.file_stem().unwrap_or_default().to_string_lossy();
    let manager = shard_manager::ShardManager::new()?; // Use manager for checks
    if manager.shard_is_protected(&shard_name_for_check) {
         // Allow modification only if owned or allowed (implement stricter check if needed)
         // For now, simple protection check:
        log_error(&format!("Cannot modify protected shard: {}", shard_name_for_check));
        return Err(ShardError::Protected(shard_name_for_check.to_string()));
    }

    let mut added_packages_map: HashMap<String, PackageType> = HashMap::new(); // Track what was added and its type

    for package_name in packages {
        // Check if already present (simplified check)
        let is_formula = manifest.formulae.contains(package_name);
        let is_cask = manifest.casks.contains(package_name);

        if is_formula {
            log_warning(&format!("Package '{}' already exists in shard as a formula. Skipping.", package_name));
            continue;
        }
        if is_cask {
            log_warning(&format!("Package '{}' already exists in shard as a cask. Skipping.", package_name));
            continue;
        }

        // Determine package type using BrewClient
        log_debug(&format!("Checking availability for '{}'", package_name));
        let availability = brew_client.check_package_availability(package_name)?;

        let determined_type = determine_package_type(package_name, &availability, force_formula, force_cask)?;

        if let Some(package_type) = determined_type {
             log_debug(&format!("Adding '{}' as {} to shard '{}'", package_name, package_type.as_str(), manifest_name));

             // Add to the appropriate list
             match package_type {
                 PackageType::Formula => {
                      manifest.formulae.push(package_name.clone());
                 }
                 PackageType::Cask => {
                      manifest.casks.push(package_name.clone());
                 }
             }
            added_packages_map.insert(package_name.clone(), package_type);
        } else {
            // determine_package_type already printed error/skip message
        }
    }

    // Save manifest if changes were made
    if !added_packages_map.is_empty() {
        if !dry_run {
            log_step(&format!("Saving updated manifest: {}", manifest_path));
            manifest.to_file(&manifest_path_obj)?;
            log_success("Manifest saved.");
        } else {
            log_debug("Dry run: Would have saved manifest.");
        }

        // --- Handle --exec and --apply ---
        if exec && !dry_run {
            log_step("Executing immediate install for added packages...");
            for (name, pkg_type) in &added_packages_map {
                 match pkg_type {
                      PackageType::Formula => brew_client.install_formula(name, &[])?,
                      PackageType::Cask => brew_client.install_cask(name, &[])?,
                 }
            }
            log_success("Immediate installation complete.");
        } else if apply_all && !dry_run {
            log_step("Running 'apply all'...");
            apply::apply_all_enabled_shards(false)?; // Don't skip cleanup
            log_success("'apply all' complete.");
        } else if exec && dry_run {
             log_debug("Dry run: Would execute immediate install for added packages.");
        } else if apply_all && dry_run {
             log_debug("Dry run: Would run 'apply all'.");
        }

    } else {
        log_debug("No new packages were added to the manifest.");
    }

    Ok(())
}

/// Helper to determine package type based on availability and flags
fn determine_package_type(
    package_name: &str,
    availability: &PackageAvailability,
    force_formula: bool,
    force_cask: bool,
) -> ShardResult<Option<PackageType>> {
    if force_formula {
        if availability.available_as_formula {
            Ok(Some(PackageType::Formula))
        } else {
            log_warning(&format!(
                "Package '{}' requested as formula, but not found as formula. {} available as cask.",
                package_name,
                if availability.available_as_cask { "It is" } else { "Not" }
            ));
            Ok(None) // Don't automatically switch if forced
        }
    } else if force_cask {
        if availability.available_as_cask {
            Ok(Some(PackageType::Cask))
        } else {
            log_warning(&format!(
                "Package '{}' requested as cask, but not found as cask. {} available as formula.",
                package_name,
                if availability.available_as_formula { "It is" } else { "Not" }
            ));
            Ok(None) // Don't automatically switch if forced
        }
    } else {
        // Auto-detect: Prefer Cask if available, otherwise Formula
        if availability.available_as_cask {
             log_debug(&format!("Package '{}' found as cask (preferred).", package_name));
            Ok(Some(PackageType::Cask))
        } else if availability.available_as_formula {
             log_debug(&format!("Package '{}' found as formula.", package_name));
            Ok(Some(PackageType::Formula))
        } else {
            log_error(&format!("Package '{}' not found as formula or cask.", package_name));
            Ok(None)
        }
    }
}

/// Remove packages from manifest and potentially uninstall/apply
pub fn remove_packages(
    packages: &[String],
    force_formula: bool, // Renamed from force_brew
    force_cask: bool,
    manifest_target: &str, // "all", name, or path
    dry_run: bool,
    exec: bool,          // New flag
    apply_all: bool,     // New flag
) -> ShardResult<()> {
    log_step(&format!("Removing packages from shard '{}': {}", manifest_target, packages.join(", ")));
    if dry_run { log_debug("Dry run enabled"); }
    if exec { log_debug("Exec flag enabled: will uninstall removed packages immediately"); }
    if apply_all { log_debug("Apply flag enabled: will run 'apply all' after removing"); }

    // Handle "all" differently - need to process multiple manifests
    if manifest_target.eq_ignore_ascii_case("all") {
        return remove_packages_from_all(packages, force_formula, force_cask, dry_run, exec, apply_all);
    }

    let brew_client = get_client();

    // Validate all package names first
    for package in packages {
        validation::validate_package_name(package)
            .with_context(|| format!("Invalid package name: {}", package))?;
    }

    // Resolve the manifest path
    let manifest_path = fs_utils::resolve_manifest_path(manifest_target)?;
    let manifest_path_obj = PathBuf::from(&manifest_path);

    // Check if the manifest exists
    if !fs_utils::path_exists(&manifest_path_obj) {
        log_error(&format!("Manifest '{}' not found. Cannot remove packages.", manifest_path));
        return Err(ShardError::NotFound(manifest_target.to_string()));
    }

    // Load the manifest
    let mut manifest = Manifest::from_file(&manifest_path_obj)
        .with_context(|| format!("Failed to load manifest: {}", manifest_path))?;
    
    // Check protection
    let shard_name_for_check = manifest_path_obj.file_stem().unwrap_or_default().to_string_lossy();
    let manager = shard_manager::ShardManager::new()?;
    if manager.shard_is_protected(&shard_name_for_check) {
        log_error(&format!("Cannot modify protected shard: {}", shard_name_for_check));
        return Err(ShardError::Protected(shard_name_for_check.to_string()));
    }

    // Track removed packages for --exec option
    let mut removed_packages: HashMap<String, PackageTypeWrapper> = HashMap::new();
    
    // Process each package for removal
    for package_name in packages {
        let mut package_found = false;
        let mut package_type = None;
        
        // Check formula sections
        if force_formula || !force_cask {
            // Check for the package in the formulas list and remove if found
            if let Some(pos) = manifest.formulae.iter().position(|f| f == package_name) {
                if !dry_run {
                    manifest.formulae.remove(pos);
                }
                package_found = true;
                package_type = Some(PackageTypeWrapper::Formula);
                log_debug(&format!("Removed formula '{}' from manifest", package_name));
            }
        }
        
        // Check cask sections
        if force_cask || (!force_formula && !package_found) {
            // Check for the package in the casks list and remove if found
            if let Some(pos) = manifest.casks.iter().position(|c| c == package_name) {
                if !dry_run {
                    manifest.casks.remove(pos);
                }
                package_found = true;
                package_type = Some(PackageTypeWrapper::Cask);
                log_debug(&format!("Removed cask '{}' from manifest", package_name));
            }
        }
        
        // If found and removed, track for potential uninstall
        if package_found {
            if let Some(pkg_type) = package_type {
                removed_packages.insert(package_name.clone(), pkg_type);
            }
        } else {
            log_warning(&format!("Package '{}' not found in shard. Skipping.", package_name));
        }
    }

    // Save the manifest if changes were made
    if !removed_packages.is_empty() {
        if !dry_run {
            log_step(&format!("Saving updated manifest: {}", manifest_path));
            manifest.to_file(&manifest_path_obj)?;
            log_success("Manifest saved.");
        } else {
            log_debug("Dry run: Would have saved manifest.");
        }
        
        // Handle --exec and --apply
        if exec && !dry_run {
            log_step("Executing immediate uninstall for removed packages...");
            
            for (name, pkg_type) in &removed_packages {
                match pkg_type {
                    PackageTypeWrapper::Formula => {
                        log_debug(&format!("Uninstalling formula: {}", name));
                        brew_client.uninstall_formula(name, true)
                            .unwrap_or_else(|e| log_error(&format!("Failed to uninstall formula {}: {}", name, e)));
                    }
                    PackageTypeWrapper::Cask => {
                        log_debug(&format!("Uninstalling cask: {}", name));
                        brew_client.uninstall_cask(name, true)
                            .unwrap_or_else(|e| log_error(&format!("Failed to uninstall cask {}: {}", name, e)));
                    }
                }
            }
            
            log_success("Immediate uninstallation complete.");
        } else if apply_all && !dry_run {
            log_step("Running 'apply all'...");
            apply::apply_all_enabled_shards(false)?;
            log_success("'apply all' complete.");
        } else if exec && dry_run {
            log_debug("Dry run: Would execute immediate uninstall for removed packages.");
        } else if apply_all && dry_run {
            log_debug("Dry run: Would run 'apply all'.");
        }
    } else {
        log_debug("No packages were removed from the manifest.");
    }
    
    Ok(())
}

/// Remove packages from all non-protected manifests
fn remove_packages_from_all(
    packages: &[String],
    force_formula: bool,
    force_cask: bool,
    dry_run: bool,
    exec: bool,
    apply_all: bool,
) -> ShardResult<()> {
    log_step(&format!("Removing packages from all non-protected shards: {}", packages.join(", ")));
    
    let manager = shard_manager::ShardManager::new()?;
    let active_shards = manager.list_shards()?;
    
    // Collect all non-protected shard paths
    let mut non_protected_shards = Vec::new();
    for shard_name in active_shards {
        if !manager.shard_is_protected(&shard_name) {
            non_protected_shards.push(shard_name);
        } else {
            log_debug(&format!("Skipping protected shard: {}", shard_name));
        }
    }
    
    if non_protected_shards.is_empty() {
        log_warning("No active, non-protected shards found to remove packages from.");
        return Ok(());
    }
    
    log_debug(&format!("Found {} non-protected shards to process", non_protected_shards.len()));
    
    // Process each shard individually
    let mut any_changes = false;
    for shard_name in non_protected_shards {
        // Use the individual shard version with exec=false to avoid multiple uninstalls
        // We'll handle exec once at the end if needed
        match remove_packages(packages, force_formula, force_cask, &shard_name, dry_run, false, false) {
            Ok(_) => {
                any_changes = true;
                log_debug(&format!("Successfully processed shard: {}", shard_name));
            },
            Err(e) => {
                log_error(&format!("Error processing shard {}: {}", shard_name, e));
                // Continue with other shards even if one fails
            }
        }
    }
    
    // After processing all shards, handle --exec and --apply if requested
    if any_changes {
        let brew_client = get_client();
        
        if exec && !dry_run {
            log_step("Executing immediate uninstall for removed packages...");
            for package_name in packages {
                // Since we don't know the types that were removed across all shards,
                // try to uninstall both types (formula and cask) if the flags allow
                if !force_cask {
                    log_debug(&format!("Attempting to uninstall formula: {}", package_name));
                    brew_client.uninstall_formula(package_name, true)
                        .unwrap_or_else(|e| log_debug(&format!("Formula {} uninstall skipped: {}", package_name, e)));
                }
                
                if !force_formula {
                    log_debug(&format!("Attempting to uninstall cask: {}", package_name));
                    brew_client.uninstall_cask(package_name, true)
                        .unwrap_or_else(|e| log_debug(&format!("Cask {} uninstall skipped: {}", package_name, e)));
                }
            }
            log_success("Immediate uninstallation attempts complete.");
        } else if apply_all && !dry_run {
            log_step("Running 'apply all'...");
            apply::apply_all_enabled_shards(false)?;
            log_success("'apply all' complete.");
        } else if exec && dry_run {
            log_debug("Dry run: Would execute immediate uninstall for removed packages.");
        } else if apply_all && dry_run {
            log_debug("Dry run: Would run 'apply all'.");
        }
    } else {
        log_debug("No packages were removed from any manifest.");
    }
    
    Ok(())
}