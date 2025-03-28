use crate::utils::{ShardResult, log_step, log_debug};
use crate::core::manifest::{Manifest, PackageState, Formula, Cask};
use crate::brew::get_client;
use crate::package::processor::{PackageProcessor, PackageType};
use std::collections::{HashSet, HashMap};
use std::path::{Path, PathBuf};
use shellexpand;
use crate::utils::filesystem;

/// Check for differences between manifest and installed packages
/// This replaces the functionality previously in apply --dry-run
pub fn diff(path: &str) -> ShardResult<()> {
    // Handle "all" special case
    if path.to_lowercase() == "all" {
        return diff_all_enabled_shards();
    }
    
    // Resolve the shard name to a proper path
    let manifest_path = filesystem::resolve_manifest_path(path)?;
    log_step(&format!("Checking changes that would be made by applying: {}", manifest_path));
    
    // Get the manifest
    let manifest_path_obj = Path::new(&manifest_path);
    let manifest = Manifest::from_file(manifest_path_obj)?;
    
    // Call internal function to perform the diff
    diff_manifest(&manifest, true)  // true for additive_only for single shard
}

/// Check for differences across all enabled shards
pub fn diff_all_enabled_shards() -> ShardResult<()> {
    log_step("Checking changes that would be made by applying all enabled shards");

    let shards_dir_path = PathBuf::from(shellexpand::tilde("~/.sapphire/shards").into_owned());

    if !std::path::Path::new(&shards_dir_path).exists() {
        log_debug("Shards directory (~/.sapphire/shards) not found. Nothing to apply.");
        return Ok(());
    }

    // --- Collect all manifests and desired state ---
    let mut all_manifests = Vec::new();
    let mut desired_taps = HashSet::new();
    let mut desired_formulae: HashMap<String, (PackageState, Vec<String>)> = HashMap::new(); // name -> (state, options)
    let mut desired_casks: HashMap<String, (PackageState, Vec<String>)> = HashMap::new();

    let entries = std::fs::read_dir(&shards_dir_path)?;

    let mut shard_files = Vec::new();
    for entry_res in entries {
        if let Ok(entry) = entry_res {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "toml") {
                shard_files.push(path);
            }
        }
    }
    shard_files.sort(); // Consistent order

    if shard_files.is_empty() {
        log_debug("No shard files (.toml) found in shards directory. Nothing to apply.");
        return Ok(());
    }

    log_step(&format!("Found {} shard file(s). Checking changes...", shard_files.len()));

    for path in &shard_files {
        match Manifest::from_file(path) {
            Ok(manifest) => {
                // Collect taps (these are strings, not structured objects)
                manifest.taps.iter().for_each(|tap| { desired_taps.insert(tap.clone()); });
                
                // Also check taps_structured if they exist
                manifest.taps_structured.iter().for_each(|tap| { desired_taps.insert(tap.name.clone()); });

                // Collect formulae (structured and simple lists)
                manifest.formulas.iter().for_each(|f| {
                    let entry = desired_formulae.entry(f.name.clone()).or_insert((f.state.clone(), f.options.clone()));
                    // Prioritize 'Latest' state if seen multiple times
                    if f.state == PackageState::Latest && entry.0 != PackageState::Latest { entry.0 = PackageState::Latest; }
                    // Merge options? For now, first non-empty options win.
                    if entry.1.is_empty() && !f.options.is_empty() { entry.1 = f.options.clone(); }
                });
                
                // Simple formulae list
                manifest.formulae.iter().for_each(|name| {
                    // Add only if not already processed from structured list
                    desired_formulae.entry(name.clone()).or_insert((PackageState::Latest, Vec::new()));
                });

                // Collect casks (structured and simple lists)
                manifest.casks_structured.iter().for_each(|c| {
                    let entry = desired_casks.entry(c.name.clone()).or_insert((c.state.clone(), c.options.clone()));
                    if c.state == PackageState::Latest && entry.0 != PackageState::Latest { entry.0 = PackageState::Latest; }
                    if entry.1.is_empty() && !c.options.is_empty() { entry.1 = c.options.clone(); }
                });
                
                // Simple casks list
                manifest.casks.iter().for_each(|name| {
                    desired_casks.entry(name.clone()).or_insert((PackageState::Latest, Vec::new()));
                });

                all_manifests.push(manifest);
            }
            Err(e) => {
                log_debug(&format!("Skipping invalid manifest file {}: {}", path.display(), e));
            }
        }
    }

    if all_manifests.is_empty() {
        log_debug("No valid manifests loaded. Nothing to apply.");
        return Ok(());
    }

    // --- Create a single "virtual" manifest representing the combined desired state ---
    let mut combined_manifest = Manifest::new();
    
    // Convert tap names to strings
    combined_manifest.taps = desired_taps.into_iter().collect();

    // Convert formulae map to structured Formula objects
    combined_manifest.formulas = desired_formulae.into_iter()
        .map(|(name, (state, options))| Formula {
            name,
            state,
            options,
            version: "latest".to_string(),
        })
        .collect();

    // Convert casks map to structured Cask objects
    combined_manifest.casks_structured = desired_casks.into_iter()
        .map(|(name, (state, options))| Cask {
            name,
            state,
            options,
            version: "latest".to_string(),
        })
        .collect();

    // Sort for consistent output/processing
    combined_manifest.taps.sort();
    combined_manifest.formulas.sort_by(|a, b| a.name.cmp(&b.name));
    combined_manifest.casks_structured.sort_by(|a, b| a.name.cmp(&b.name));

    // Perform the diff for the combined manifest
    diff_manifest(&combined_manifest, false) // false for additive_only for "all" shards
}

/// Internal function to diff a manifest against the current system state
fn diff_manifest(manifest: &Manifest, additive_only: bool) -> ShardResult<()> {
    let brew_client = get_client();

    // --- Process Taps ---
    if !manifest.taps.is_empty() {
        log_step(&format!("Checking {} taps...", manifest.taps.len()));
        let installed_taps = brew_client.get_installed_taps()?;
        
        for tap in &manifest.taps {
            if installed_taps.contains(tap) {
                log_debug(&format!("✅ Tap already installed: {}", tap));
            } else {
                log_step(&format!("❌ Tap would be installed: {}", tap));
            }
        }
    }

    // --- Process Formulas & Casks ---
    let installed_formulae = brew_client.get_installed_formulae()?;
    let installed_casks = brew_client.get_installed_casks()?;

    // Create processors
    let formula_processor = PackageProcessor::new(PackageType::Formula, installed_formulae.clone(), true);
    let cask_processor = PackageProcessor::new(PackageType::Cask, installed_casks.clone(), true);

    // Process packages using the processors - check both structured and simple lists
    // For individual shards, formulae (simple string list) is the primary storage
    let total_formulae_count = manifest.formulae.len() + manifest.formulas.len();
    log_step(&format!("Checking {} formulae...", total_formulae_count));
    
    // Process structured formulas
    let formula_ops = formula_processor.process_packages(&manifest.formulas)?;
    
    // Process simple formulae list (added via shard add)
    let formulae_ops = formula_processor.process_packages(&manifest.formulae)?;
    
    // Combine to-install lists
    let mut combined_formulae_to_install = formula_ops.to_install.clone();
    combined_formulae_to_install.extend(formulae_ops.to_install);
    
    if !combined_formulae_to_install.is_empty() {
        log_step(&format!("Would install {} formula(s):", combined_formulae_to_install.len()));
        for formula in &combined_formulae_to_install {
            log_step(&format!("  • {}", formula));
        }
    }
    
    // Handle with_options for both
    for (name, options) in &formula_ops.with_options {
        // Only show installation messages for packages not already installed
        if !formula_processor.is_installed(name) {
            log_step(&format!("Would install formula {} with options: {}", name, options.join(" ")));
        }
    }
    for (name, options) in &formulae_ops.with_options {
        // Only show installation messages for packages not already installed
        if !formula_processor.is_installed(name) {
            log_step(&format!("Would install formula {} with options: {}", name, options.join(" ")));
        }
    }
    
    // Combine to-uninstall lists
    let mut combined_formulae_to_uninstall = formula_ops.to_uninstall.clone();
    combined_formulae_to_uninstall.extend(formulae_ops.to_uninstall);
    
    if !combined_formulae_to_uninstall.is_empty() {
        log_step(&format!("Would uninstall {} formula(s):", combined_formulae_to_uninstall.len()));
        for formula in &combined_formulae_to_uninstall {
            log_step(&format!("  • {}", formula));
        }
    }

    // Process casks - handle both structured and simple lists
    let total_casks_count = manifest.casks.len() + manifest.casks_structured.len();
    log_step(&format!("Checking {} casks...", total_casks_count));
    
    // Process structured casks
    let cask_ops = cask_processor.process_packages(&manifest.casks_structured)?;
    
    // Process simple casks list (added via shard add)
    let casks_ops = cask_processor.process_packages(&manifest.casks)?;
    
    // Combine to-install lists
    let mut combined_casks_to_install = cask_ops.to_install.clone();
    combined_casks_to_install.extend(casks_ops.to_install);
    
    if !combined_casks_to_install.is_empty() {
        log_step(&format!("Would install {} cask(s):", combined_casks_to_install.len()));
        for cask in &combined_casks_to_install {
            log_step(&format!("  • {}", cask));
        }
    }
    
    // Handle with_options for both
    for (name, options) in &cask_ops.with_options {
        // Only show installation messages for packages not already installed
        if !cask_processor.is_installed(name) {
            log_step(&format!("Would install cask {} with options: {}", name, options.join(" ")));
        }
    }
    for (name, options) in &casks_ops.with_options {
        // Only show installation messages for packages not already installed
        if !cask_processor.is_installed(name) {
            log_step(&format!("Would install cask {} with options: {}", name, options.join(" ")));
        }
    }
    
    // Combine to-uninstall lists
    let mut combined_casks_to_uninstall = cask_ops.to_uninstall.clone();
    combined_casks_to_uninstall.extend(casks_ops.to_uninstall);
    
    if !combined_casks_to_uninstall.is_empty() {
        log_step(&format!("Would uninstall {} cask(s):", combined_casks_to_uninstall.len()));
        for cask in &combined_casks_to_uninstall {
            log_step(&format!("  • {}", cask));
        }
    }

    // --- Process Implied Uninstalls (only if not additive and this is an "all" operation) ---
    if !additive_only {
        log_step("Checking for packages to uninstall (not present in any shard)...");

        // Get all *main* packages currently installed (exclude dependencies)
        let (main_formulae, main_casks) = match get_all_main_packages() {
            Ok(packages) => packages,
            Err(e) => {
                log_debug(&format!("Error getting installed packages: {}", e));
                return Ok(());
            }
        };

        // Identify formulae defined in the manifest (packages we want to keep or manage)
        // Include both structured and simple lists
        let mut desired_formulae_names: HashSet<String> = manifest.formulas.iter()
            .filter(|f| f.state != PackageState::Absent) // Only count packages meant to be present/latest
            .map(|f| f.name.clone())
            .collect();
        
        // Add formulae from simple list
        desired_formulae_names.extend(manifest.formulae.iter().cloned());

        // Identify casks defined in the manifest (both structured and simple lists)
        let mut desired_casks_names: HashSet<String> = manifest.casks_structured.iter()
            .filter(|c| c.state != PackageState::Absent)
            .map(|c| c.name.clone())
            .collect();
            
        // Add casks from simple list
        desired_casks_names.extend(manifest.casks.iter().cloned());

        // Find packages installed but not desired anymore
        let formulae_to_uninstall: Vec<_> = main_formulae.iter()
            .filter(|name| !desired_formulae_names.contains(*name))
            .cloned()
            .collect();

        let casks_to_uninstall: Vec<_> = main_casks.iter()
            .filter(|name| !desired_casks_names.contains(*name))
            .cloned()
            .collect();

        if !formulae_to_uninstall.is_empty() {
            log_step(&format!("Would uninstall {} formula(s):", formulae_to_uninstall.len()));
            for formula in &formulae_to_uninstall {
                log_step(&format!("  • {}", formula));
            }
        }

        if !casks_to_uninstall.is_empty() {
            log_step(&format!("Would uninstall {} cask(s):", casks_to_uninstall.len()));
            for cask in &casks_to_uninstall {
                log_step(&format!("  • {}", cask));
            }
        }
    }

    // --- Cleanup ---
    log_debug("Would run cleanup if needed");

    Ok(())
}

/// Helper function to get all main packages (not dependencies)
fn get_all_main_packages() -> ShardResult<(Vec<String>, Vec<String>)> {
    let brew_client = get_client();
    
    // Get all installed packages
    let formulae = brew_client.get_installed_formulae()?;
    let casks = brew_client.get_installed_casks()?;
    
    // Get dependency packages (these will be excluded)
    let dependencies = brew_client.get_dependency_packages()?;
    
    // Filter out dependencies
    let main_formulae: Vec<String> = formulae
        .into_iter()
        .filter(|f| !dependencies.contains(f))
        .collect();
    
    // Casks are typically not dependencies, but filter for consistency
    let main_casks: Vec<String> = casks
        .into_iter()
        .filter(|c| !dependencies.contains(c))
        .collect();
    
    Ok((main_formulae, main_casks))
}