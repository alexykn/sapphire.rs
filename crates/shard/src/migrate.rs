use anyhow::{Context, Result, bail};
use console::style;
use dialoguer::{Confirm, Select};
use shellexpand;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::process::Command;
use crate::manifest::{Manifest, Formula, Cask, Tap, PackageState};
use std::path::Path;
use std::fs;
use serde_yaml;
use sapphire_core::utils::file_system as fs_utils;
use regex::Regex;

/// Migrate packages from a Nix configuration file to a new custom shard
pub fn migrate_from_nix(
    system_nix: Option<&str>,
    user_nix: Option<&str>,
    custom_name: Option<&str>,
    non_interactive: bool,
    dry_run: bool,
) -> Result<()> {
    if system_nix.is_none() && user_nix.is_none() {
        bail!("No input files specified. Please provide at least one Nix file with --system_apps or --user_apps");
    }

    println!("{} from Nix configuration to a new custom shard", style("Migrating packages").bold().green());
    
    // Create a single manifest for all migrated packages
    let mut custom_manifest = Manifest::new();
    custom_manifest.metadata.description = "Packages migrated from Nix configuration".to_string();
    custom_manifest.metadata.protected = false; // Custom shard is unlocked
    
    // Process system-level configuration if provided
    if let Some(system_path) = system_nix {
        println!("Processing system configuration: {}", style(system_path).bold());
        let system_config = read_nix_file(system_path)
            .context("Failed to read system Nix configuration file")?;
        
        process_nix_config(&system_config, &mut custom_manifest, non_interactive)
            .context("Failed to process system Nix configuration")?;
    }
    
    // Process user-level configuration if provided
    if let Some(user_path) = user_nix {
        println!("Processing user configuration: {}", style(user_path).bold());
        let user_config = read_nix_file(user_path)
            .context("Failed to read user Nix configuration file")?;
        
        process_nix_config(&user_config, &mut custom_manifest, non_interactive)
            .context("Failed to process user Nix configuration")?;
    }
    
    // Print summary for the custom shard
    println!("\n{}", style("Migration Summary:").bold().underlined());
    print_migration_summary(&custom_manifest);
    
    // Save the manifest unless it's a dry run
    if !dry_run {
        // Determine the name for the output shard
        let output_name = if let Some(name) = custom_name {
            name.to_string()
        } else if system_nix.is_some() && user_nix.is_some() {
            "nix_migrate_combined".to_string()
        } else if system_nix.is_some() {
            "nix_migrate_system".to_string()
        } else {
            "nix_migrate_user".to_string()
        };
        
        // Filter out Mac App Store entries that we can't handle yet
        custom_manifest.taps.retain(|tap| !tap.name.contains('='));
        custom_manifest.formulas.retain(|formula| !formula.name.contains('='));
        custom_manifest.casks.retain(|cask| !cask.name.contains('='));
        
        // Convert structured taps/formulas/casks to simplified arrays
        // This ensures the TOML file will use the simplified format consistently
        custom_manifest.formulae.clear();
        for formula in &custom_manifest.formulas {
            if formula.state == PackageState::Present || formula.state == PackageState::Latest {
                if formula.version == "latest" {
                    custom_manifest.formulae.push(formula.name.clone());
                } else {
                    custom_manifest.formulae.push(format!("{}:{}", formula.name, formula.version));
                }
            }
        }
        
        custom_manifest.brews.clear();
        for cask in &custom_manifest.casks {
            if cask.state == PackageState::Present || cask.state == PackageState::Latest {
                if cask.version == "latest" {
                    custom_manifest.brews.push(cask.name.clone());
                } else {
                    custom_manifest.brews.push(format!("{}:{}", cask.name, cask.version));
                }
            }
        }
        
        // Ensure the shards directory exists
        std::fs::create_dir_all(shellexpand::tilde("~/.sapphire/shards").to_string())
            .context("Failed to create shards directory")?;
        
        let custom_path = format!("~/.sapphire/shards/{}.toml", output_name);
        let custom_path = &shellexpand::tilde(&custom_path).to_string();
        custom_manifest.to_file(custom_path)
            .context("Failed to save custom shard")?;
        println!("Custom shard saved to: {}", style(custom_path).bold());
        println!("{} Shard created and is ready to use", 
            style("✓").bold().green());
    } else {
        println!("{} No changes made (dry run)", style("✓").bold().green());
    }
    
    Ok(())
}

/// Read and parse a Nix configuration file
fn read_nix_file(path: &str) -> Result<String> {
    let path = shellexpand::tilde(path).to_string();
    let mut file = File::open(&path)
        .with_context(|| format!("Failed to open Nix file: {}", path))?;
    
    let mut content = String::new();
    file.read_to_string(&mut content)
        .with_context(|| format!("Failed to read Nix file: {}", path))?;
    
    Ok(content)
}

/// Process Nix configuration and extract package information
fn process_nix_config(config: &str, manifest: &mut Manifest, non_interactive: bool) -> Result<()> {
    // Extract the homebrew section
    if let Some(homebrew_section) = extract_section(config, "homebrew") {
        // Process taps
        if let Some(taps_section) = extract_section(&homebrew_section, "taps") {
            let taps = extract_array_items(&taps_section);
            for tap in taps {
                // Skip if already added (to prevent duplicates)
                if !manifest.taps.iter().any(|t| t.name == tap) {
                    manifest.taps.push(Tap { name: tap });
                }
            }
        }
        
        // Process brews (formulas) - check both direct brews and nested homebrew.brews
        let brew_sections = vec![
            extract_section(&homebrew_section, "brews"),
            extract_section(config, "homebrew.brews"),
        ];
        
        for opt_section in brew_sections {
            if let Some(brews_section) = opt_section {
                let brews = extract_array_items(&brews_section);
                for brew in brews {
                    // Skip if already added (to prevent duplicates)
                    if !manifest.formulas.iter().any(|f| f.name == brew) {
                        manifest.formulas.push(Formula {
                            name: brew,
                            version: "latest".to_string(),
                            options: Vec::new(),
                            state: PackageState::Present,
                        });
                    }
                }
            }
        }
        
        // Process casks - check both direct casks and nested homebrew.casks
        let cask_sections = vec![
            extract_section(&homebrew_section, "casks"),
            extract_section(config, "homebrew.casks"),
        ];
        
        for opt_section in cask_sections {
            if let Some(casks_section) = opt_section {
                let casks = extract_array_items(&casks_section);
                for cask in casks {
                    // Skip if already added (to prevent duplicates)
                    if !manifest.casks.iter().any(|c| c.name == cask) {
                        manifest.casks.push(Cask {
                            name: cask,
                            version: "latest".to_string(),
                            options: Vec::new(),
                            state: PackageState::Present,
                        });
                    }
                }
            }
        }
        
        // Process masApps
        if let Some(_) = extract_section(&homebrew_section, "masApps") {
            // In the future, we'll handle Mac App Store apps
            // Currently not supported in the manifest format
            println!("{} Mac App Store apps detected but not yet supported", style("⚠").bold().yellow());
        }
    }
    
    // Extract system packages from environment.systemPackages
    if let Some(sys_packages_section) = extract_section(config, "environment.systemPackages") {
        let packages = extract_array_items(&sys_packages_section);
        
        // Try to find potential Homebrew package equivalents
        for pkg in packages {
            // Skip complex expressions like callPackage
            if pkg.contains("(") || pkg.contains("{") || pkg.contains("./") {
                continue;
            }
            
            // Check if this package might be available as a Homebrew formula
            match verify_package_availability(&pkg) {
                PackageMigration::Brew(brew_name) => {
                    // Skip if already added (to prevent duplicates)
                    if !manifest.formulas.iter().any(|f| f.name == brew_name) {
                        manifest.formulas.push(Formula {
                            name: brew_name,
                            version: "latest".to_string(),
                            options: Vec::new(),
                            state: PackageState::Present,
                        });
                    }
                }
                PackageMigration::Cask(cask_name) => {
                    // Skip if already added (to prevent duplicates)
                    if !manifest.casks.iter().any(|c| c.name == cask_name) {
                        manifest.casks.push(Cask {
                            name: cask_name,
                            version: "latest".to_string(),
                            options: Vec::new(),
                            state: PackageState::Present,
                        });
                    }
                }
                PackageMigration::Suggestion(suggestions) => {
                    // Only offer suggestions for actual package names, not syntax elements
                    if !pkg.trim().is_empty() && 
                       !pkg.contains("with pkgs") && 
                       !pkg.contains("};") {
                        
                        // First, check for exact matches and add them automatically
                        let exact_match = suggestions.iter().find(|s| 
                            (s.name.to_lowercase() == pkg.to_lowercase() || s.similarity > 0.95)
                        );
                        
                        if let Some(exact) = exact_match {
                            // We have an exact match, add it automatically
                            match exact.package_type {
                                PackageType::Brew => {
                                    // Skip if already added (to prevent duplicates)
                                    if !manifest.formulas.iter().any(|f| f.name == exact.name) {
                                        manifest.formulas.push(Formula {
                                            name: exact.name.clone(),
                                            version: "latest".to_string(),
                                            options: Vec::new(),
                                            state: PackageState::Present,
                                        });
                                        println!("{} Auto-added exact formula match: {} for {}",
                                            style("✓").bold().green(), 
                                            style(&exact.name).bold(),
                                            style(&pkg).bold());
                                    }
                                }
                                PackageType::Cask => {
                                    // Skip if already added (to prevent duplicates)
                                    if !manifest.casks.iter().any(|c| c.name == exact.name) {
                                        manifest.casks.push(Cask {
                                            name: exact.name.clone(),
                                            version: "latest".to_string(),
                                            options: Vec::new(),
                                            state: PackageState::Present,
                                        });
                                        println!("{} Auto-added exact cask match: {} for {}",
                                            style("✓").bold().green(), 
                                            style(&exact.name).bold(),
                                            style(&pkg).bold());
                                    }
                                }
                            }
                            continue;
                        }
                        
                        // No exact match found, continue with regular process
                        // Skip interactive suggestions if in non-interactive mode
                        if non_interactive {
                            // Use the best suggestion automatically if it has high similarity
                            if !suggestions.is_empty() && suggestions[0].similarity > 0.8 {
                                let best = &suggestions[0];
                                match best.package_type {
                                    PackageType::Brew => {
                                        // Skip if already added (to prevent duplicates)
                                        if !manifest.formulas.iter().any(|f| f.name == best.name) {
                                            manifest.formulas.push(Formula {
                                                name: best.name.clone(),
                                                version: "latest".to_string(),
                                                options: Vec::new(),
                                                state: PackageState::Present,
                                            });
                                            println!("{} Auto-added formula: {} ({}% match for {})", 
                                                style("✓").bold().green(), 
                                                style(&best.name).bold(),
                                                (best.similarity * 100.0) as u8,
                                                style(&pkg).bold());
                                        }
                                    }
                                    PackageType::Cask => {
                                        // Skip if already added (to prevent duplicates)
                                        if !manifest.casks.iter().any(|c| c.name == best.name) {
                                            manifest.casks.push(Cask {
                                                name: best.name.clone(),
                                                version: "latest".to_string(),
                                                options: Vec::new(),
                                                state: PackageState::Present,
                                            });
                                            println!("{} Auto-added cask: {} ({}% match for {})", 
                                                style("✓").bold().green(), 
                                                style(&best.name).bold(),
                                                (best.similarity * 100.0) as u8,
                                                style(&pkg).bold());
                                        }
                                    }
                                }
                            } else {
                                println!("{} No close match found for Nix package: {} (use interactive mode for suggestions)",
                                    style("⚠").bold().yellow(), style(&pkg).bold());
                            }
                        } else {
                            // Interactive mode - Display the suggestions to the user
                            println!("\n{} for Nix package: {}", 
                                style("Suggestions").bold().green(), 
                                style(&pkg).bold());
                            
                            let mut items = Vec::new();
                            for suggestion in &suggestions {
                                let pkg_type = match suggestion.package_type {
                                    PackageType::Brew => "formula",
                                    PackageType::Cask => "cask",
                                };
                                let similarity = (suggestion.similarity * 100.0) as u8;
                                items.push(format!("{} ({}%, {})", 
                                    suggestion.name, similarity, pkg_type));
                            }
                            items.push("Skip this package".to_string());
                            
                            // Ask the user to select an option
                            if let Ok(selection) = Select::new()
                                .with_prompt("Select a Homebrew equivalent")
                                .items(&items)
                                .default(items.len() - 1) // Default to "Skip"
                                .interact() {
                                
                                // If user didn't select "Skip"
                                if selection < suggestions.len() {
                                    let selected = &suggestions[selection];
                                    match selected.package_type {
                                        PackageType::Brew => {
                                            // Skip if already added (to prevent duplicates)
                                            if !manifest.formulas.iter().any(|f| f.name == selected.name) {
                                                manifest.formulas.push(Formula {
                                                    name: selected.name.clone(),
                                                    version: "latest".to_string(),
                                                    options: Vec::new(),
                                                    state: PackageState::Present,
                                                });
                                                println!("{} Added formula: {}", 
                                                    style("✓").bold().green(), 
                                                    style(&selected.name).bold());
                                            }
                                        }
                                        PackageType::Cask => {
                                            // Skip if already added (to prevent duplicates)
                                            if !manifest.casks.iter().any(|c| c.name == selected.name) {
                                                manifest.casks.push(Cask {
                                                    name: selected.name.clone(),
                                                    version: "latest".to_string(),
                                                    options: Vec::new(),
                                                    state: PackageState::Present,
                                                });
                                                println!("{} Added cask: {}", 
                                                    style("✓").bold().green(), 
                                                    style(&selected.name).bold());
                                            }
                                        }
                                    }
                                } else {
                                    println!("{} Skipped package: {}", 
                                        style("Note").bold().yellow(), 
                                        style(&pkg).bold());
                                }
                            }
                        }
                    }
                }
                PackageMigration::NotFound => {
                    // Only print warning for actual package names, not syntax elements
                    if !pkg.trim().is_empty() && 
                       !pkg.contains("with pkgs") && 
                       !pkg.contains("};") {
                        println!("{} Could not find Homebrew equivalent for Nix package: {}",
                            style("⚠").bold().yellow(), style(&pkg).bold());
                    }
                }
            }
        }
    }
    
    Ok(())
}

/// Represents the result of trying to find a Homebrew equivalent for a Nix package
enum PackageMigration {
    Brew(String),
    Cask(String),
    Suggestion(Vec<PackageSuggestion>),
    NotFound,
}

/// A suggested package with type and similarity score
struct PackageSuggestion {
    name: String,
    package_type: PackageType,
    similarity: f32,
}

/// Package type (cask or formula)
#[derive(Clone)]
enum PackageType {
    Brew,
    Cask,
}

/// Attempt to verify if a Nix package has a Homebrew equivalent
fn verify_package_availability(package: &str) -> PackageMigration {
    // Skip if package is empty or looks like a syntax element
    let package = package.trim();
    if package.is_empty() || 
       package.contains("with pkgs") || 
       package.contains("};") ||
       package.contains("[") || 
       package.contains("]") {
        return PackageMigration::NotFound;
    }
    
    // Small set of common direct mappings that we're very confident about
    // This is only for the most common and obvious matches to avoid brew search overhead
    let mut direct_mappings = HashMap::new();
    
    // Core utilities and common tools - formulas
    direct_mappings.insert("vim", ("vim", PackageType::Brew));
    direct_mappings.insert("neovim", ("neovim", PackageType::Brew));
    direct_mappings.insert("helix", ("helix", PackageType::Brew));
    direct_mappings.insert("node", ("node", PackageType::Brew));
    direct_mappings.insert("nodejs", ("node", PackageType::Brew));
    direct_mappings.insert("python", ("python", PackageType::Brew));
    direct_mappings.insert("python3", ("python", PackageType::Brew));
    direct_mappings.insert("go", ("go", PackageType::Brew));
    direct_mappings.insert("rust", ("rust", PackageType::Brew));
    direct_mappings.insert("rustup", ("rustup", PackageType::Brew));
    direct_mappings.insert("bat", ("bat", PackageType::Brew));
    direct_mappings.insert("ripgrep", ("ripgrep", PackageType::Brew));
    direct_mappings.insert("eza", ("eza", PackageType::Brew));
    direct_mappings.insert("fd", ("fd", PackageType::Brew));
    direct_mappings.insert("fzf", ("fzf", PackageType::Brew));
    direct_mappings.insert("zoxide", ("zoxide", PackageType::Brew));
    direct_mappings.insert("delta", ("git-delta", PackageType::Brew));
    direct_mappings.insert("starship", ("starship", PackageType::Brew));
    direct_mappings.insert("btop", ("btop", PackageType::Brew));
    direct_mappings.insert("htop", ("htop", PackageType::Brew));
    direct_mappings.insert("ffmpeg", ("ffmpeg", PackageType::Brew));
    direct_mappings.insert("mpv", ("mpv", PackageType::Brew));
    direct_mappings.insert("ansible", ("ansible", PackageType::Brew));
    direct_mappings.insert("lazygit", ("lazygit", PackageType::Brew));
    direct_mappings.insert("cowsay", ("cowsay", PackageType::Brew));
    
    // Common applications - casks
    direct_mappings.insert("firefox", ("firefox", PackageType::Cask));
    direct_mappings.insert("google-chrome", ("google-chrome", PackageType::Cask));
    direct_mappings.insert("visual-studio-code", ("visual-studio-code", PackageType::Cask));
    direct_mappings.insert("vscode", ("visual-studio-code", PackageType::Cask));
    direct_mappings.insert("cursor", ("cursor", PackageType::Cask));
    direct_mappings.insert("beekeeper-studio", ("beekeeper-studio", PackageType::Cask));
    direct_mappings.insert("chromedriver", ("chromedriver", PackageType::Cask));
    direct_mappings.insert("orbstack", ("orbstack", PackageType::Cask));
    direct_mappings.insert("ghostty", ("ghostty", PackageType::Cask));
    direct_mappings.insert("signal", ("signal", PackageType::Cask));
    direct_mappings.insert("microsoft-outlook", ("microsoft-outlook", PackageType::Cask));
    direct_mappings.insert("obsidian", ("obsidian", PackageType::Cask));
    direct_mappings.insert("updf", ("updf", PackageType::Cask));
    direct_mappings.insert("discord", ("discord", PackageType::Cask));
    direct_mappings.insert("vlc", ("vlc", PackageType::Cask));
    direct_mappings.insert("dbeaver-community", ("dbeaver-community", PackageType::Cask));
    direct_mappings.insert("claude", ("claude", PackageType::Cask));
    
    // First check for exact matches in our direct mappings
    if let Some((name, pkg_type)) = direct_mappings.get(package) {
        match pkg_type {
            PackageType::Brew => return PackageMigration::Brew(name.to_string()),
            PackageType::Cask => return PackageMigration::Cask(name.to_string()),
        }
    }
    
    // For packages not in our direct mappings, use `brew search` to find suggestions
    match find_package_suggestions(package) {
        Ok(suggestions) if !suggestions.is_empty() => {
            // If the best suggestion has very high similarity, use it directly
            if suggestions[0].similarity > 0.9 {
                match suggestions[0].package_type {
                    PackageType::Brew => return PackageMigration::Brew(suggestions[0].name.clone()),
                    PackageType::Cask => return PackageMigration::Cask(suggestions[0].name.clone()),
                }
            }
            
            // Otherwise return all suggestions for interactive selection
            PackageMigration::Suggestion(suggestions)
        },
        _ => PackageMigration::NotFound,
    }
}

/// Calculate similarity between two strings (simple Levenshtein-based approach)
fn calculate_similarity(s1: &str, s2: &str) -> f32 {
    let s1_lower = s1.to_lowercase();
    let s2_lower = s2.to_lowercase();
    
    // If one string contains the other, high similarity
    if s1_lower.contains(&s2_lower) || s2_lower.contains(&s1_lower) {
        return 0.9;
    }
    
    // Calculate Levenshtein distance
    let distance = levenshtein_distance(&s1_lower, &s2_lower);
    let max_len = std::cmp::max(s1_lower.len(), s2_lower.len());
    
    if max_len == 0 {
        return 1.0;
    }
    
    1.0 - (distance as f32 / max_len as f32)
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    
    let m = s1_chars.len();
    let n = s2_chars.len();
    
    // Handle empty strings
    if m == 0 { return n; }
    if n == 0 { return m; }
    
    // Create distance matrix
    let mut matrix = vec![vec![0; n + 1]; m + 1];
    
    // Initialize first row and column
    for i in 0..=m {
        matrix[i][0] = i;
    }
    
    for j in 0..=n {
        matrix[0][j] = j;
    }
    
    // Fill in the rest of the matrix
    for i in 1..=m {
        for j in 1..=n {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] { 0 } else { 1 };
            matrix[i][j] = std::cmp::min(
                std::cmp::min(
                    matrix[i - 1][j] + 1,      // deletion
                    matrix[i][j - 1] + 1       // insertion
                ),
                matrix[i - 1][j - 1] + cost    // substitution
            );
        }
    }
    
    matrix[m][n]
}

/// Find Homebrew package suggestions for a Nix package using brew search
fn find_package_suggestions(package_name: &str) -> Result<Vec<PackageSuggestion>> {
    let mut suggestions = Vec::new();
    
    // Run brew search command
    let output = Command::new("brew")
        .arg("search")
        .arg(package_name)
        .output()
        .with_context(|| format!("Failed to execute brew search for '{}'", package_name))?;
    
    if !output.status.success() {
        return Ok(Vec::new()); // Return empty if search failed
    }
    
    let search_results = String::from_utf8_lossy(&output.stdout).to_string();
    let search_lines: Vec<&str> = search_results.lines().collect();
    
    // Process search results
    for line in search_lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        
        // Skip headers and non-package lines
        if line.starts_with("==>") || line.contains("No formula or cask found") {
            continue;
        }
        
        // Determine if it's a cask or formula
        let (name, package_type) = if line.contains(" (Cask)") {
            let name = line.replace(" (Cask)", "").trim().to_string();
            (name, PackageType::Cask)
        } else {
            (line.to_string(), PackageType::Brew)
        };
        
        // PRIORITY 1: Exact match gets highest priority
        if name.to_lowercase() == package_name.to_lowercase() {
            // Exact match - give it the highest score and add it first
            suggestions.insert(0, PackageSuggestion {
                name: name.clone(),
                package_type,
                similarity: 1.0,
            });
            continue;
        }
        
        // Calculate similarity for non-exact matches
        let similarity = calculate_similarity(package_name, &name);
        
        // Add to suggestions if similarity is reasonable
        if similarity > 0.3 {
            suggestions.push(PackageSuggestion {
                name,
                package_type,
                similarity,
            });
        }
    }
    
    // If we still don't have any suggestions, try direct name as a formula
    if suggestions.is_empty() {
        suggestions.push(PackageSuggestion {
            name: package_name.to_string(),
            package_type: PackageType::Brew, // Default to formula
            similarity: 0.9,
        });
    }
    
    // Sort by similarity score (highest first)
    suggestions.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
    
    // Return top 3 suggestions, but with a twist: if there's an exact match, ensure it's first
    let mut results = Vec::new();
    
    for suggestion in suggestions.into_iter().take(3) {
        // If we find an exact match (or very close), put it first
        if suggestion.name.to_lowercase() == package_name.to_lowercase() {
            results.insert(0, suggestion);
        } else {
            results.push(suggestion);
        }
    }
    
    Ok(results)
}

/// Extract a section from Nix configuration based on a key
fn extract_section(content: &str, section_name: &str) -> Option<String> {
    // Special handling for environment.systemPackages with pkgs syntax
    if section_name == "environment.systemPackages" {
        // Pattern like: environment.systemPackages = with pkgs; [
        let with_pkgs_pattern = "environment.systemPackages = with pkgs;";
        if let Some(start_idx) = content.find(with_pkgs_pattern) {
            let after_pattern = &content[start_idx + with_pkgs_pattern.len()..];
            let trimmed = after_pattern.trim();
            
            // Check if it starts with a bracket
            if trimmed.starts_with('[') {
                let mut bracket_depth = 0;
                let mut section_content = String::new();
                let mut capture = false;
                
                for c in trimmed.chars() {
                    if c == '[' {
                        bracket_depth += 1;
                        if bracket_depth == 1 {
                            capture = true;
                            continue; // Skip the opening bracket
                        }
                    } else if c == ']' {
                        bracket_depth -= 1;
                        if bracket_depth == 0 {
                            break; // End of section
                        }
                    }
                    
                    if capture {
                        section_content.push(c);
                    }
                }
                
                return Some(section_content);
            }
        }
    }
    
    // Check for direct key matches (section_name = {...}, section_name = [...])
    let section_pattern = format!("{} = ", section_name);
    
    if let Some(start_idx) = content.find(&section_pattern) {
        let section_start = start_idx + section_pattern.len();
        let remaining = &content[section_start..];
        
        // Handle object sections with curly braces
        if remaining.trim().starts_with('{') {
            let mut brace_count = 0;
            let mut in_section = false;
            let mut section_content = String::new();
            
            for (_, c) in remaining.char_indices() {
                if !in_section && c == '{' {
                    in_section = true;
                    brace_count += 1;
                    continue;
                }
                
                if in_section {
                    if c == '{' {
                        brace_count += 1;
                    } else if c == '}' {
                        brace_count -= 1;
                        if brace_count == 0 {
                            break;
                        }
                    }
                    section_content.push(c);
                }
            }
            
            return Some(section_content);
        }
        
        // Handle array sections with square brackets
        if remaining.trim().starts_with('[') {
            let mut bracket_count = 0;
            let mut in_section = false;
            let mut section_content = String::new();
            
            for (_, c) in remaining.char_indices() {
                if !in_section && c == '[' {
                    in_section = true;
                    bracket_count += 1;
                    continue;
                }
                
                if in_section {
                    if c == '[' {
                        bracket_count += 1;
                    } else if c == ']' {
                        bracket_count -= 1;
                        if bracket_count == 0 {
                            break;
                        }
                    }
                    section_content.push(c);
                }
            }
            
            return Some(section_content);
        }
    }
    
    // Handle nested keys like "homebrew.brews" by splitting and traversing
    if section_name.contains('.') {
        let parts: Vec<&str> = section_name.splitn(2, '.').collect();
        if parts.len() == 2 {
            let parent = parts[0];
            let child = parts[1];
            
            if let Some(parent_content) = extract_section(content, parent) {
                return extract_section(&parent_content, child);
            }
        }
    }
    
    None
}

/// Extract items from an array-like section in Nix configuration
fn extract_array_items(array_content: &str) -> Vec<String> {
    let mut items = Vec::new();
    
    // Split by newlines and process each line
    for line in array_content.lines() {
        let trimmed = line.trim();
        
        // Skip empty lines, comments, or structural elements
        if trimmed.is_empty() || 
           trimmed.starts_with("#") || 
           trimmed == "[" || 
           trimmed == "]" {
            continue;
        }
        
        // Skip lines that appear to be section headers or complex expressions
        if (trimmed.contains(" = ") && !trimmed.contains("\"")) || 
           (trimmed.contains("=") && trimmed.contains("{")) ||
           trimmed == "with pkgs;" ||
           trimmed.contains("environment.systemPackages") {
            continue;
        }
        
        // Extract the item name, handling quotes and trailing comma
        if let Some(item) = extract_item_name(trimmed) {
            // Skip structural keywords or empty items
            if !item.is_empty() && 
               item != "brews" && 
               item != "casks" && 
               item != "taps" && 
               !item.contains("=") {
                items.push(item);
            }
        }
    }
    
    items
}

/// Extract an item name from a line of Nix configuration
fn extract_item_name(line: &str) -> Option<String> {
    let mut item = line.trim();
    
    // Remove trailing semicolon or comma if present
    if item.ends_with(';') || item.ends_with(',') {
        item = &item[0..item.len() - 1];
    }
    
    // Handle special case for with pkgs syntax
    if item.starts_with("with pkgs;") {
        return None;
    }
    
    // Handle package expressions like "pkgs.ripgrep"
    if item.starts_with("pkgs.") {
        item = &item["pkgs.".len()..];
    }
    
    // Remove quotes if present
    if (item.starts_with('"') && item.ends_with('"')) || 
       (item.starts_with('\'') && item.ends_with('\'')) {
        item = &item[1..item.len() - 1];
    }
    
    // Handle the case where there might be a expression/function call
    if let Some(paren_idx) = item.find('(') {
        item = &item[0..paren_idx].trim();
    }
    
    if item.is_empty() {
        None
    } else {
        Some(item.to_string())
    }
}

/// Print a summary of the migration results
fn print_migration_summary(manifest: &Manifest) {
    println!("\n{}", style("Migration Summary:").bold().underlined());
    
    println!("  {}: {}", style("Taps").bold(), manifest.taps.len());
    println!("  {}: {}", style("Formulas").bold(), manifest.formulas.len());
    println!("  {}: {}", style("Casks").bold(), manifest.casks.len());
    
    if !manifest.taps.is_empty() {
        println!("\n{}", style("Taps:").bold());
        for tap in &manifest.taps {
            // Skip items that look like Mac App Store entries
            if !tap.name.contains('=') {
                println!("  - {}", tap.name);
            }
        }
    }
    
    if !manifest.formulas.is_empty() {
        println!("\n{}", style("Formulas:").bold());
        for formula in &manifest.formulas {
            // Skip items that look like Mac App Store entries
            if !formula.name.contains('=') {
                println!("  - {}", formula.name);
            }
        }
    }
    
    if !manifest.casks.is_empty() {
        println!("\n{}", style("Casks:").bold());
        for cask in &manifest.casks {
            // Skip items that look like Mac App Store entries
            if !cask.name.contains('=') {
                println!("  - {}", cask.name);
            }
        }
    }
    
    // Print a note if we don't have MAS support yet but MAS apps were found
    let has_mas_entries = 
        manifest.taps.iter().any(|t| t.name.contains('=')) ||
        manifest.formulas.iter().any(|f| f.name.contains('=')) ||
        manifest.casks.iter().any(|c| c.name.contains('='));
    
    if has_mas_entries {
        println!("\n{} Found Mac App Store apps, but they're not yet supported.", 
            style("Note:").bold().yellow());
        println!("  They will be skipped in the generated shard.");
    }
}

/// Convert old YAML shard files to new TOML format
pub fn convert_yaml_to_toml(force: bool) -> Result<()> {
    println!("Converting YAML shards to TOML format...");
    
    // Get path to shards directory
    let shards_dir = shellexpand::tilde("~/.sapphire/shards").to_string();
    
    // Create shards directory if it doesn't exist
    if !fs_utils::path_exists(&shards_dir) {
        println!("No shards directory found. Nothing to convert.");
        return Ok(());
    }
    
    // Read directory
    let entries = fs::read_dir(&shards_dir)
        .with_context(|| format!("Failed to read shards directory: {}", shards_dir))?;
    
    let mut converted = 0;
    let mut failed = 0;
    
    // Process each YAML file
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        // Only process .yaml files
        if !path.is_file() || path.extension().map_or(true, |ext| ext != "yaml") {
            continue;
        }
        
        // Generate TOML path
        let file_stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let toml_path = path.with_file_name(format!("{}.toml", file_stem));
        
        // Check if destination already exists and we're not forcing
        if toml_path.exists() && !force {
            println!("Skipping {} (TOML file already exists)", path.display());
            continue;
        }
        
        // Try to read and convert the file
        match convert_single_file(&path, &toml_path) {
            Ok(_) => {
                println!("Converted {} to {}", path.display(), toml_path.display());
                converted += 1;
                
                // Remove the original YAML file
                if let Err(e) = fs::remove_file(&path) {
                    println!("Warning: Could not remove original YAML file: {}", e);
                }
            },
            Err(e) => {
                println!("Failed to convert {}: {}", path.display(), e);
                failed += 1;
            }
        }
    }
    
    println!("Conversion complete: {} files converted, {} failed", converted, failed);
    
    Ok(())
}

/// Convert a single YAML file to TOML
fn convert_single_file(yaml_path: &Path, toml_path: &Path) -> Result<()> {
    // Read the YAML file
    let file = std::fs::File::open(yaml_path)
        .with_context(|| format!("Failed to open YAML file: {}", yaml_path.display()))?;
    
    // Parse the YAML
    let mut manifest: Manifest = serde_yaml::from_reader(file)
        .with_context(|| format!("Failed to parse YAML file: {}", yaml_path.display()))?;
    
    // Convert structured formats to simplified arrays for consistency
    manifest.formulae.clear();
    for formula in &manifest.formulas {
        if formula.state == PackageState::Present || formula.state == PackageState::Latest {
            if formula.version == "latest" {
                manifest.formulae.push(formula.name.clone());
            } else {
                manifest.formulae.push(format!("{}:{}", formula.name, formula.version));
            }
        }
    }
    
    manifest.brews.clear();
    for cask in &manifest.casks {
        if cask.state == PackageState::Present || cask.state == PackageState::Latest {
            if cask.version == "latest" {
                manifest.brews.push(cask.name.clone());
            } else {
                manifest.brews.push(format!("{}:{}", cask.name, cask.version));
            }
        }
    }
    
    // Write as TOML
    manifest.to_file(toml_path)?;
    
    Ok(())
}