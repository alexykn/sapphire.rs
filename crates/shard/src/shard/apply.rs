use anyhow::{Context, Result};
use std::path::Path;
use std::fs;
use std::io::Write;
use console::style;
use shellexpand;
use crate::core::manifest::{Manifest, PackageState};
use sapphire_core::utils::file_system as fs_utils;
use crate::package::processor::{
    PackageType, PackageOperation, PackageProcessor,
    get_installed_formulae, get_installed_casks, get_installed_taps, get_dependency_packages,
    add_tap, run_cleanup, uninstall_package,
    get_all_main_packages
};

/// Options for applying manifests
#[derive(Debug, Default, Clone)]
pub struct ApplyOptions {
    /// Whether to allow removal of packages from the system manifest
    pub allow_system_package_removal: bool,
    /// Whether to suppress redundant "already installed" messages
    pub quiet_mode: bool,
}

/// Process taps defined in a manifest
fn process_taps(manifest: &Manifest, dry_run: bool) -> Result<()> {
    if manifest.taps.is_empty() {
        return Ok(());
    }
    
    println!("Processing {} taps...", manifest.taps.len());
    
    if dry_run {
        // Dry run output
        for tap in &manifest.taps {
            println!("Would add tap: {}", tap.name);
        }
        return Ok(());
    }
    
    // Get currently installed taps
    let installed_taps = get_installed_taps()
        .with_context(|| "Failed to retrieve installed taps")?;
    
    // Add taps that are not already tapped
    let taps_to_add: Vec<_> = manifest.taps.iter()
        .filter(|tap| !installed_taps.contains(&tap.name))
        .collect();
    
    if taps_to_add.is_empty() {
        println!("All required taps are already installed");
    } else {
        for tap in taps_to_add {
            add_tap(&tap.name)?;
        }
    }
    
    Ok(())
}

/// Handle cleanup operations
fn handle_cleanup(skip_cleanup: bool, dry_run: bool) -> Result<()> {
    if !skip_cleanup {
        if dry_run {
            println!("Would run cleanup");
        } else {
            // Use the run_cleanup function instead of direct client call
            run_cleanup()?;
        }
    }
    
    Ok(())
}

/// Apply a single manifest
pub fn apply_internal(manifest: &Manifest, dry_run: bool, skip_cleanup: bool) -> Result<()> {
    // Internal implementation with default options
    apply_internal_with_options(manifest, dry_run, skip_cleanup, &ApplyOptions::default())
}

/// Apply a single manifest with custom options
pub fn apply_internal_with_options(
    manifest: &Manifest, 
    dry_run: bool, 
    skip_cleanup: bool,
    options: &ApplyOptions
) -> Result<()> {
    // Process taps
    process_taps(manifest, dry_run)?;
    
    // Process formulas and casks
    let is_system_manifest = false; // For single manifest, assume not system manifest
    process_formulas(manifest, dry_run, is_system_manifest, options)?;
    process_casks(manifest, dry_run, is_system_manifest, options)?;
    
    // Cleanup if not skipped and not dry run
    handle_cleanup(skip_cleanup, dry_run)?;
    
    Ok(())
}

/// Process formulas defined in a manifest
fn process_formulas(
    manifest: &Manifest,
    dry_run: bool,
    is_system_manifest: bool, 
    options: &ApplyOptions
) -> Result<()> {
    // Check if manifest has formulas
    if manifest.formulas.is_empty() && manifest.formulae.is_empty() {
        return Ok(());
    }

    // Create a vector to hold all formula information
    let mut formula_info = Vec::new();
    
    // Add detailed formulas first
    for formula in &manifest.formulas {
        if formula.state != PackageState::Absent {
            formula_info.push((
                formula.name.clone(),
                formula.state.clone(),
                formula.options.clone()
            ));
        }
    }
    
    // Add simplified formula list (formulae)
    for formula_name in &manifest.formulae {
        // Only add if not already in formula_info from the detailed list
        if !formula_info.iter().any(|(name, _, _)| name == formula_name) {
            formula_info.push((
                formula_name.clone(),
                PackageState::Latest,  // Simplified list implies Latest state
                Vec::new()  // No options for simplified list
            ));
        }
    }
    
    if formula_info.is_empty() {
        // No formulas to install or upgrade
        // Check for absent formulas to uninstall
        let absent_formulas: Vec<String> = manifest.formulas.iter()
            .filter(|f| f.state == PackageState::Absent)
            .map(|f| f.name.clone())
            .collect();
        
        if absent_formulas.is_empty() {
            return Ok(());
        }
        
        if !options.quiet_mode {
            println!("Processing {} formulae for removal...", absent_formulas.len());
        }
        
        if !is_system_manifest || options.allow_system_package_removal {
            let installed_packages = get_installed_formulae()?;
            
            for formula_name in &absent_formulas {
                if installed_packages.contains(formula_name) {
                    if dry_run {
                        println!("Would uninstall formula: {}", style(formula_name).bold());
                    } else {
                        println!("{} formula: {}", 
                            PackageOperation::Uninstall.as_str(), 
                            style(formula_name).bold());
                        uninstall_package(PackageType::Formula, formula_name, true)?;
                    }
                }
            }
        } else {
            println!("{} {} {}",
                style("Skipping uninstall of").yellow(),
                absent_formulas.len(),
                style("formulas from system manifest").yellow());
        }
        
        return Ok(());
    }
    
    if !options.quiet_mode {
        println!("Processing {} formulae...", formula_info.len());
    }
    
    // Create formula processor
    let formula_processor = PackageProcessor {
        package_type: PackageType::Formula,
        installed_packages: get_installed_formulae()?,
        suppress_messages: options.quiet_mode,
    };

    // Create SimplePackage objects for processing
    struct SimplePackage {
        name: String,
        state: PackageState,
        options: Vec<String>,
    }
    
    impl crate::package::processor::PackageInfo for SimplePackage {
        fn state(&self) -> PackageState {
            self.state.clone()
        }
        
        fn options(&self) -> &[String] {
            &self.options
        }
        
        fn name(&self) -> &str {
            &self.name
        }
    }
    
    let formula_packages: Vec<SimplePackage> = formula_info
        .into_iter()
        .map(|(name, state, options)| SimplePackage { name, state, options })
        .collect();
        
    // Process packages with the processor
    let process_result = formula_processor.process_packages(&formula_packages)?;
    formula_processor.execute_operations(&process_result, dry_run)?;
    
    // Process "absent" formulas separately
    let absent_formulas: Vec<String> = manifest.formulas.iter()
        .filter(|f| f.state == PackageState::Absent)
        .map(|f| f.name.clone())
        .collect();
    
    if !absent_formulas.is_empty() {
        if !is_system_manifest || options.allow_system_package_removal {
            for formula_name in &absent_formulas {
                if formula_processor.installed_packages.contains(formula_name) {
                    if dry_run {
                        println!("Would uninstall formula: {}", style(formula_name).bold());
                    } else {
                        println!("{} formula: {}", 
                            PackageOperation::Uninstall.as_str(), 
                            style(formula_name).bold());
                        uninstall_package(PackageType::Formula, formula_name, true)?;
                    }
                }
            }
        } else {
            println!("{} {} {}",
                style("Skipping uninstall of").yellow(),
                absent_formulas.len(),
                style("formulas from system manifest").yellow());
        }
    }
    
    Ok(())
}

/// Process casks defined in a manifest
fn process_casks(
    manifest: &Manifest,
    dry_run: bool,
    is_system_manifest: bool, 
    options: &ApplyOptions
) -> Result<()> {
    // Check if manifest has casks
    if manifest.casks.is_empty() && manifest.brews.is_empty() {
        return Ok(());
    }

    // Create a vector to hold all cask information
    let mut cask_info = Vec::new();
    
    // Add detailed casks first
    for cask in &manifest.casks {
        if cask.state != PackageState::Absent {
            cask_info.push((
                cask.name.clone(),
                cask.state.clone(),
                cask.options.clone()
            ));
        }
    }
    
    // Add simplified cask list (brews)
    for cask_name in &manifest.brews {
        // Only add if not already in cask_info from the detailed list
        if !cask_info.iter().any(|(name, _, _)| name == cask_name) {
            cask_info.push((
                cask_name.clone(),
                PackageState::Latest,  // Simplified list implies Latest state
                Vec::new()  // No options for simplified list
            ));
        }
    }
    
    if cask_info.is_empty() {
        // No casks to install or upgrade
        // Check for absent casks to uninstall
        let absent_casks: Vec<String> = manifest.casks.iter()
            .filter(|c| c.state == PackageState::Absent)
            .map(|c| c.name.clone())
            .collect();
        
        if absent_casks.is_empty() {
            return Ok(());
        }
        
        if !options.quiet_mode {
            println!("Processing {} casks for removal...", absent_casks.len());
        }
        
        if !is_system_manifest || options.allow_system_package_removal {
            let installed_packages = get_installed_casks()?;
            
            for cask_name in &absent_casks {
                if installed_packages.contains(cask_name) {
                    if dry_run {
                        println!("Would uninstall cask: {}", style(cask_name).bold());
                    } else {
                        println!("{} cask: {}", 
                            PackageOperation::Uninstall.as_str(), 
                            style(cask_name).bold());
                        uninstall_package(PackageType::Cask, cask_name, true)?;
                    }
                }
            }
        } else {
            println!("{} {} {}",
                style("Skipping uninstall of").yellow(),
                absent_casks.len(),
                style("casks from system manifest").yellow());
        }
        
        return Ok(());
    }
    
    if !options.quiet_mode {
        println!("Processing {} casks...", cask_info.len());
    }
    
    // Create cask processor
    let cask_processor = PackageProcessor {
        package_type: PackageType::Cask,
        installed_packages: get_installed_casks()?,
        suppress_messages: options.quiet_mode,
    };

    // Create SimplePackage objects for processing
    struct SimplePackage {
        name: String,
        state: PackageState,
        options: Vec<String>,
    }
    
    impl crate::package::processor::PackageInfo for SimplePackage {
        fn state(&self) -> PackageState {
            self.state.clone()
        }
        
        fn options(&self) -> &[String] {
            &self.options
        }
        
        fn name(&self) -> &str {
            &self.name
        }
    }
    
    let cask_packages: Vec<SimplePackage> = cask_info
        .into_iter()
        .map(|(name, state, options)| SimplePackage { name, state, options })
        .collect();
        
    // Process packages with the processor
    let process_result = cask_processor.process_packages(&cask_packages)?;
    cask_processor.execute_operations(&process_result, dry_run)?;
    
    // Process "absent" casks separately
    let absent_casks: Vec<String> = manifest.casks.iter()
        .filter(|c| c.state == PackageState::Absent)
        .map(|c| c.name.clone())
        .collect();
    
    if !absent_casks.is_empty() {
        if !is_system_manifest || options.allow_system_package_removal {
            for cask_name in &absent_casks {
                if cask_processor.installed_packages.contains(cask_name) {
                    if dry_run {
                        println!("Would uninstall cask: {}", style(cask_name).bold());
                    } else {
                        println!("{} cask: {}", 
                            PackageOperation::Uninstall.as_str(), 
                            style(cask_name).bold());
                        uninstall_package(PackageType::Cask, cask_name, true)?;
                    }
                }
            }
        } else {
            println!("{} {} {}",
                style("Skipping uninstall of").yellow(),
                absent_casks.len(),
                style("casks from system manifest").yellow());
        }
    }
    
    Ok(())
}

/// Apply a specific shard manifest file
pub fn apply(shard: &str, dry_run: bool, skip_cleanup: bool) -> Result<()> {
    // Construct the path to the shard file
    let shards_dir = shellexpand::tilde("~/.sapphire/shards").to_string();
    let manifest_path = format!("{}/{}.toml", shards_dir, shard);
    
    // Check if the file exists
    if !fs_utils::path_exists(&manifest_path) {
        anyhow::bail!("Shard file not found: {}", manifest_path);
    }
    
    // Load the manifest
    let manifest = Manifest::from_file(Path::new(&manifest_path))
        .with_context(|| format!("Failed to load manifest file: {}", manifest_path))?;
    
    // Apply the manifest
    apply_internal(&manifest, dry_run, skip_cleanup)
}

/// Apply all enabled shards in the shards directory
pub fn apply_all_enabled_shards(dry_run: bool, skip_cleanup: bool) -> Result<()> {
    // Internal function call with default behavior
    apply_all_enabled_shards_internal(dry_run, skip_cleanup, true)
}

/// Process implied uninstalls for packages not present in any manifest
fn process_implied_uninstalls(
    processed_formula_names: &[String], 
    processed_cask_names: &[String],
    dry_run: bool
) -> Result<()> {
    if dry_run {
        println!("Would check for implied uninstalls (packages not in any manifest)");
        return Ok(());
    }
    
    // Get the list of main packages (explicitly installed, not dependencies)
    let (main_formulae, main_casks) = get_all_main_packages()
        .with_context(|| "Failed to retrieve main packages")?;
    
    // Find formulae to uninstall
    let formulae_to_uninstall: Vec<_> = main_formulae.iter()
        .filter(|name| !processed_formula_names.contains(name))
        .collect();
    
    // Find casks to uninstall
    let casks_to_uninstall: Vec<_> = main_casks.iter()
        .filter(|name| !processed_cask_names.contains(name))
        .collect();
    
    // Process formulae uninstallation
    if !formulae_to_uninstall.is_empty() {
        println!("Uninstalling {} formulae not present in any enabled shard...", 
            formulae_to_uninstall.len());
        
        for name in formulae_to_uninstall {
            uninstall_package(PackageType::Formula, name, true)?;
        }
    }
    
    // Process casks uninstallation
    if !casks_to_uninstall.is_empty() {
        println!("Uninstalling {} casks not present in any enabled shard...", 
            casks_to_uninstall.len());
        
        for name in casks_to_uninstall {
            uninstall_package(PackageType::Cask, name, true)?;
        }
    }
    
    Ok(())
}

/// Internal implementation of apply_all_enabled_shards with control over exit behavior
fn apply_all_enabled_shards_internal(dry_run: bool, skip_cleanup: bool, should_exit: bool) -> Result<()> {
    println!("{} Applying all enabled shards", style("Sapphire").bold().green());
    
    // Get path to shards directory
    let shards_dir = shellexpand::tilde("~/.sapphire/shards").to_string();
    
    // Check if shards directory exists
    if !fs_utils::path_exists(&shards_dir) {
        println!("No shards directory found. Nothing to apply.");
        return Ok(());
    }
    
    // Read all yaml files in the directory
    let entries = fs::read_dir(&shards_dir)
        .with_context(|| format!("Failed to read shards directory: {}", shards_dir))?;
    
    // Collect paths to process
    let shard_paths: Vec<_> = entries
        .filter_map(|entry_result| {
            entry_result.ok().and_then(|entry| {
                let path = entry.path();
                // Keep only non-directory toml files
                if path.is_dir() || path.extension().map_or(true, |ext| ext != "toml") {
                    None
                } else {
                    Some(path)
                }
            })
        })
        .collect();
    
    // Sort by filename to ensure consistent order
    let mut sorted_paths = shard_paths.clone();
    sorted_paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    
    if sorted_paths.is_empty() {
        println!("No shards found in directory. Nothing to apply.");
        return Ok(());
    }
    
    // Get lists of currently installed packages
    println!("Gathering currently installed packages...");
    let installed_formulae = get_installed_formulae()
        .with_context(|| "Failed to retrieve installed formulae")?;
    let installed_casks = get_installed_casks()
        .with_context(|| "Failed to retrieve installed casks")?;
    
    // Create package processors with message suppression
    let formula_processor = PackageProcessor::new(
        PackageType::Formula, 
        installed_formulae.clone(), 
        true  // Suppress "already installed" messages
    );
    
    let cask_processor = PackageProcessor::new(
        PackageType::Cask, 
        installed_casks.clone(), 
        true  // Suppress "already installed" messages
    );
    
    // Load all manifests and collect desired packages
    println!("Loading shard manifests...");
    
    // Create structures to hold package information
    let mut all_manifests = Vec::new();
    let mut desired_taps = Vec::new();
    let mut formula_info = Vec::new();
    let mut cask_info = Vec::new();
    
    // Process each manifest
    for path in &sorted_paths {
        let manifest = Manifest::from_file(path)
            .with_context(|| format!("Failed to load manifest file: {}", path.display()))?;
        
        // Collect taps
        desired_taps.extend(manifest.taps.iter().map(|tap| tap.name.clone()));
        
        // Collect formulae
        for formula in &manifest.formulas {
            if formula.state != PackageState::Absent {
                formula_info.push((
                    formula.name.clone(), 
                    formula.state.clone(), 
                    formula.options.clone()
                ));
            }
        }
        
        // Collect casks
        for cask in &manifest.casks {
            if cask.state != PackageState::Absent {
                cask_info.push((
                    cask.name.clone(), 
                    cask.state.clone(), 
                    cask.options.clone()
                ));
            }
        }
        
        all_manifests.push(manifest);
    }
    
    // Remove duplicates from desired taps
    desired_taps.sort();
    desired_taps.dedup();
    
    // Helper function to deduplicate packages with priority for Latest state
    fn deduplicate_packages(
        packages: Vec<(String, PackageState, Vec<String>)>
    ) -> Vec<(String, PackageState, Vec<String>)> {
        let mut result = Vec::new();
        let mut processed_names = Vec::new();
        
        for (name, state, options) in packages {
            match processed_names.iter().position(|n| n == &name) {
                None => {
                    // First time seeing this package
                    processed_names.push(name.clone());
                    result.push((name, state, options));
                },
                Some(idx) => {
                    // Already have this package
                    // Update if new state is Latest and current isn't
                    if state == PackageState::Latest && result[idx].1 != PackageState::Latest {
                        result[idx].1 = PackageState::Latest;
                        // Also update options if they exist
                        if !options.is_empty() {
                            result[idx].2 = options;
                        }
                    }
                }
            }
        }
        
        result
    }
    
    // Deduplicate the package lists
    let unique_formulae = deduplicate_packages(formula_info);
    let unique_casks = deduplicate_packages(cask_info);
    
    // Extract the names of desired packages for later reference
    let processed_formula_names: Vec<String> = unique_formulae.iter()
        .map(|(name, _, _)| name.clone())
        .collect();
    
    let processed_cask_names: Vec<String> = unique_casks.iter()
        .map(|(name, _, _)| name.clone())
        .collect();
    
    // Process taps
    if !desired_taps.is_empty() {
        println!("Processing {} taps...", desired_taps.len());
        
        if !dry_run {
            // Get currently installed taps
            let installed_taps = get_installed_taps()
                .with_context(|| "Failed to retrieve installed taps")?;
            
            for tap in &desired_taps {
                if !installed_taps.contains(tap) {
                    add_tap(tap)?;
                } 
            }
        } else {
            for tap in &desired_taps {
                println!("Would add tap: {}", tap);
            }
        }
    }
    
    // Create package objects for formulae and casks to work with the processor
    struct SimplePackage {
        name: String,
        state: PackageState,
        options: Vec<String>,
    }
    
    impl crate::package::processor::PackageInfo for SimplePackage {
        fn state(&self) -> PackageState {
            self.state.clone()
        }
        
        fn options(&self) -> &[String] {
            &self.options
        }
        
        fn name(&self) -> &str {
            &self.name
        }
    }
    
    let formula_packages: Vec<SimplePackage> = unique_formulae
        .into_iter()
        .map(|(name, state, options)| SimplePackage { name, state, options })
        .collect();
    
    let cask_packages: Vec<SimplePackage> = unique_casks
        .into_iter()
        .map(|(name, state, options)| SimplePackage { name, state, options })
        .collect();
    
    // Process packages with the processors
    if !formula_packages.is_empty() {
        println!("Processing {} formulae...", formula_packages.len());
        let formula_result = formula_processor.process_packages(&formula_packages)?;
        formula_processor.execute_operations(&formula_result, dry_run)?;
    }
    
    if !cask_packages.is_empty() {
        println!("Processing {} casks...", cask_packages.len());
        let cask_result = cask_processor.process_packages(&cask_packages)?;
        cask_processor.execute_operations(&cask_result, dry_run)?;
    }
    
    // Display active packages
    println!("{} Using these packages from enabled shards:", style("ℹ").bold().blue());
    
    // Show taps
    if !desired_taps.is_empty() {
        println!("  {}", style("Active Taps:").bold());
        for tap in &desired_taps {
            println!("    - {}", style(tap).bold());
        }
    }
    
    // Show active formulae
    if !processed_formula_names.is_empty() {
        // Get dependency packages
        let dependency_packages = get_dependency_packages()
            .with_context(|| "Failed to retrieve dependency packages")?;
        
        println!("  {}", style("Active Formulae:").bold());
        for formula in &processed_formula_names {
            if !dependency_packages.contains(formula) {
                let state_info = formula_packages.iter()
                    .find(|pkg| pkg.name == *formula)
                    .map(|pkg| if pkg.state == PackageState::Latest { "(latest)" } else { "" })
                    .unwrap_or("");
                println!("    - {} {}", style(formula).bold(), state_info);
            }
        }
    }
    
    // Show active casks
    if !processed_cask_names.is_empty() {
        println!("  {}", style("Active Casks:").bold());
        for cask in &processed_cask_names {
            let state_info = cask_packages.iter()
                .find(|pkg| pkg.name == *cask)
                .map(|pkg| if pkg.state == PackageState::Latest { "(latest)" } else { "" })
                .unwrap_or("");
            println!("    - {} {}", style(cask).bold(), state_info);
        }
    }
    
    // Process implied uninstalls
    process_implied_uninstalls(&processed_formula_names, &processed_cask_names, dry_run)?;
    
    // Final cleanup if not skipped and not dry run
    handle_cleanup(skip_cleanup, dry_run)?;
    
    let total_shards = sorted_paths.len();
    println!("\n{} Applied {} shards successfully", 
        style("✓").bold().green(), total_shards);
    
    std::io::stdout().flush()
        .with_context(|| "Failed to flush stdout")?;
    
    // Final completion message
    println!("\n{} All operations complete - exiting", style("✓").bold().green());
    std::io::stdout().flush()
        .with_context(|| "Failed to flush stdout")?;
    
    // Force exit the process to ensure termination, but only when called directly
    if !dry_run && should_exit {
        // Use a more reliable approach to ensure we exit
        std::thread::sleep(std::time::Duration::from_millis(500));
        // Force exit to ensure termination
        std::process::exit(0);
    }
    
    Ok(())
}