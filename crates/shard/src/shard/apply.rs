use crate::utils::{ShardResult, ShardError, ResultExt, log_success, log_warning, log_error, log_step, log_debug};
use crate::package::processor::{PackageProcessor, PackageType};
use crate::core::manifest::{Manifest, PackageState};
use crate::brew::{get_client, client::BrewClient};
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::fs;
use shellexpand;
use crate::utils::filesystem::{path_exists, resolve_manifest_path};

/// Options for applying manifests - SIMPLIFIED
#[derive(Debug, Default, Clone)]
pub struct ApplyOptions {
    /// If true, only install/upgrade, do not uninstall anything.
    pub additive_only: bool,
    /// If true, skip the final `brew cleanup`.
    pub skip_cleanup: bool,
}

/// Apply a *single* shard manifest file (ADDITIVE ONLY)
/// Installs/upgrades packages defined in the shard, does NOT uninstall anything.
pub fn apply_single_shard(shard_name: &str, skip_cleanup: bool) -> ShardResult<()> {
    log_step(&format!("Applying single shard (additive mode): {}", shard_name));

    let manifest_path = resolve_manifest_path(shard_name)?;
    let manifest_path_obj = Path::new(&manifest_path);

    if !path_exists(manifest_path_obj) {
        log_error(&format!("Shard manifest not found: {}", manifest_path));
        return Err(ShardError::NotFound(shard_name.to_string()));
    }

    let manifest = Manifest::from_file(manifest_path_obj)
        .with_context(|| format!("Failed to load manifest: {}", manifest_path))?;

    let options = ApplyOptions {
        additive_only: true, // Force additive mode for single shard apply
        skip_cleanup,
    };

    // Call the internal apply function
    apply_manifest(&manifest, &options)
}

/// Apply *all* enabled shards (SYNCHRONIZING)
/// Installs/upgrades packages from all shards, uninstalls packages not in any enabled shard.
pub fn apply_all_enabled_shards(skip_cleanup: bool) -> ShardResult<()> {
    log_step("Applying all enabled shards (synchronizing)");

    let shards_dir_path = PathBuf::from(shellexpand::tilde("~/.sapphire/shards").into_owned());

    if !path_exists(&shards_dir_path) {
        log_warning("Shards directory (~/.sapphire/shards) not found. Nothing to apply.");
        return Ok(());
    }

    // --- 1. Collect all manifests and desired state ---
    let mut all_manifests = Vec::new();
    let mut desired_taps = HashSet::new();
    let mut desired_formulae = HashSet::new();
    let mut desired_casks = HashSet::new();

    let entries = fs::read_dir(&shards_dir_path)
        .with_context(|| format!("Failed to read shards directory: {}", shards_dir_path.display()))?;

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
        log_warning("No shard files (.toml) found in shards directory. Nothing to apply.");
        return Ok(());
    }

    log_debug(&format!("Found {} shard file(s). Loading manifests...", shard_files.len()));

    for path in &shard_files {
        match Manifest::from_file(path) {
            Ok(manifest) => {
                log_debug(&format!("Loaded shard: {}", path.display()));
                
                // Collect taps (from both simple and structured formats)
                manifest.taps.iter().for_each(|tap_name| { desired_taps.insert(tap_name.clone()); });
                manifest.taps_structured.iter().for_each(|tap| { desired_taps.insert(tap.name.clone()); });

                // Collect formulae (from both simple and structured formats)
                manifest.formulae.iter().for_each(|formula_name| { desired_formulae.insert(formula_name.clone()); });
                manifest.formulas.iter()
                    .filter(|f| f.state != PackageState::Absent) // Skip explicitly absent packages
                    .for_each(|f| { desired_formulae.insert(f.name.clone()); });

                // Collect casks (from both simple and structured formats)
                manifest.casks.iter().for_each(|cask_name| { desired_casks.insert(cask_name.clone()); });
                manifest.casks_structured.iter()
                    .filter(|c| c.state != PackageState::Absent) // Skip explicitly absent packages
                    .for_each(|c| { desired_casks.insert(c.name.clone()); });

                all_manifests.push(manifest);
            }
            Err(e) => {
                log_warning(&format!("Skipping invalid manifest file {}: {}", path.display(), e));
            }
        }
    }

    if all_manifests.is_empty() {
        log_warning("No valid manifests loaded. Nothing to apply.");
        return Ok(());
    }

    // --- 2. Create a single "virtual" manifest representing the combined desired state ---
    let mut combined_manifest = Manifest::new();
    combined_manifest.taps = desired_taps.into_iter().collect();
    combined_manifest.taps.sort(); // Sort for consistent output
    
    combined_manifest.formulae = desired_formulae.into_iter().collect();
    combined_manifest.formulae.sort(); // Sort for consistent output
    
    combined_manifest.casks = desired_casks.into_iter().collect();
    combined_manifest.casks.sort(); // Sort for consistent output

    // --- 3. Apply the combined manifest ---
    let options = ApplyOptions {
        additive_only: false, // Allow uninstalls for 'apply all'
        skip_cleanup,
    };
    apply_manifest(&combined_manifest, &options)?;

    log_success(&format!("Applied {} shards successfully.", all_manifests.len()));

    Ok(())
}

/// Internal function to apply a given manifest state (can be combined or single)
fn apply_manifest(manifest: &Manifest, options: &ApplyOptions) -> ShardResult<()> {
    let brew_client = get_client();

    // --- 1. Process Taps ---
    if !manifest.taps.is_empty() {
        log_step(&format!("Processing {} taps...", manifest.taps.len()));
        let installed_taps = brew_client.get_installed_taps()?.into_iter().collect::<HashSet<_>>();
        
        for tap in &manifest.taps {
            if !installed_taps.contains(tap) {
                brew_client.add_tap(tap)?;
            }
        }
    }

    // --- 2. Process Formulas & Casks ---
    log_debug("Gathering current system state...");
    let installed_formulae = brew_client.get_installed_formulae()?;
    let installed_casks = brew_client.get_installed_casks()?;

    // Create processors
    let formula_processor = PackageProcessor::new(PackageType::Formula, installed_formulae.clone(), true);
    let cask_processor = PackageProcessor::new(PackageType::Cask, installed_casks.clone(), true);

    // Process packages using the processors
    log_step(&format!("Processing {} formulae...", manifest.formulae.len()));
    let formula_ops = formula_processor.process_packages(&manifest.formulae)?;
    formula_processor.execute_operations(&formula_ops, false)?; // false = no dry run

    log_step(&format!("Processing {} casks...", manifest.casks.len()));
    let cask_ops = cask_processor.process_packages(&manifest.casks)?;
    cask_processor.execute_operations(&cask_ops, false)?; // false = no dry run

    // --- 3. Process Implied Uninstalls (only if not additive) ---
    if !options.additive_only {
        log_step("Checking for packages to uninstall (not present in any shard)...");

        // Get all *main* packages currently installed (exclude dependencies)
        let (main_formulae, main_casks) = get_all_main_packages(&brew_client)?;

        // Identify formulae defined in the manifest - consider all forms
        let mut desired_formulae_names = HashSet::new();
        desired_formulae_names.extend(manifest.formulae.iter().map(|s| s.as_str()));
        desired_formulae_names.extend(manifest.formulas.iter().filter_map(|f| {
            if f.state != PackageState::Absent { Some(f.name.as_str()) } else { None }
        }));

        // Identify casks defined in the manifest - consider all forms
        let mut desired_casks_names = HashSet::new();
        desired_casks_names.extend(manifest.casks.iter().map(|s| s.as_str()));
        desired_casks_names.extend(manifest.casks_structured.iter().filter_map(|c| {
            if c.state != PackageState::Absent { Some(c.name.as_str()) } else { None }
        }));

        // Get system dependencies to protect them
        let dependency_packages = brew_client.get_dependency_packages()?;
        let dependency_set: HashSet<&str> = dependency_packages.iter().map(|s| s.as_str()).collect();

        // Create a safe list of packages that shouldn't be uninstalled
        let critical_packages = vec!["git", "brew", "curl", "openssl", "python", "fish", "bash", "zsh"];
        let critical_set: HashSet<&str> = critical_packages.iter().copied().collect();

        // Find formulae to uninstall: not in manifest, not a dependency, not critical
        let formulae_to_uninstall: Vec<_> = main_formulae.iter()
            .filter(|name| {
                !desired_formulae_names.contains(name.as_str()) && 
                !dependency_set.contains(name.as_str()) &&
                !critical_set.contains(name.as_str())
            })
            .cloned()
            .collect();

        // Find casks to uninstall: not in manifest, not critical
        let casks_to_uninstall: Vec<_> = main_casks.iter()
            .filter(|name| {
                !desired_casks_names.contains(name.as_str()) && 
                !critical_set.contains(name.as_str())
            })
            .cloned()
            .collect();

        if !formulae_to_uninstall.is_empty() {
            log_debug(&format!("Found {} formulae to uninstall: {}", formulae_to_uninstall.len(), formulae_to_uninstall.join(", ")));
            for name in formulae_to_uninstall {
                log_debug(&format!("Uninstalling formula: {}", name));
                // Use BrewClient directly
                brew_client.uninstall_formula(&name, true).unwrap_or_else(|e| 
                    log_error(&format!("Failed uninstalling formula {}: {}", name, e))
                );
            }
        } else {
            log_debug("No extra formulae found to uninstall.");
        }

        if !casks_to_uninstall.is_empty() {
            log_debug(&format!("Found {} casks to uninstall: {}", casks_to_uninstall.len(), casks_to_uninstall.join(", ")));
            for name in casks_to_uninstall {
                log_debug(&format!("Uninstalling cask: {}", name));
                brew_client.uninstall_cask(&name, true).unwrap_or_else(|e| 
                    log_error(&format!("Failed uninstalling cask {}: {}", name, e))
                );
            }
        } else {
            log_debug("No extra casks found to uninstall.");
        }
    } else {
        log_debug("Additive mode: Skipping uninstallation of packages not in manifest.");
    }

    // --- 4. Cleanup ---
    if !options.skip_cleanup {
        brew_client.cleanup(true)?; // true for prune_all
    } else {
        log_debug("Skipping cleanup step.");
    }

    Ok(())
}

/// Helper function to get main packages (non-dependencies)
fn get_all_main_packages(brew_client: &BrewClient) -> ShardResult<(Vec<String>, Vec<String>)> {
    let installed_formulae = brew_client.get_installed_formulae()?;
    let installed_casks = brew_client.get_installed_casks()?;
    let dependency_packages = brew_client.get_dependency_packages()?;
    
    // Filter out dependencies from installed formulae
    let main_formulae: Vec<String> = installed_formulae
        .into_iter()
        .filter(|name| !dependency_packages.contains(name))
        .collect();
    
    // Casks are never dependencies
    let main_casks = installed_casks;
    
    Ok((main_formulae, main_casks))
}

/// Apply a manifest (backwards compatibility function)
pub fn apply(shard: &str, skip_cleanup: bool) -> ShardResult<()> {
    if shard.eq_ignore_ascii_case("all") {
        apply_all_enabled_shards(skip_cleanup)
    } else {
        apply_single_shard(shard, skip_cleanup)
    }
}