//! Homebrew package search and information functionality.
//!
//! This module handles searching for packages and retrieving package information
//! from Homebrew repositories. These operations don't modify local system state and 
//! primarily focus on discovery and information retrieval. All user inputs are properly
//! validated to prevent command injection.

use anyhow::Result;
use console::style;
use crate::brew::core::BrewCore;
use crate::brew::validate as validation;

/// Searcher for Homebrew packages
pub struct BrewSearcher {
    core: BrewCore,
}

/// Formula information structure
pub struct FormulaInfo {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// Cask information structure
pub struct CaskInfo {
    pub name: String,
    pub version: String,
    pub description: String,
}

impl BrewSearcher {
    /// Create a new searcher with default brew core
    pub fn new() -> Self {
        Self {
            core: BrewCore::new(),
        }
    }
    
    /// Create a new searcher with a custom brew core
    pub fn with_core(core: BrewCore) -> Self {
        Self { core }
    }

    /// Search for Homebrew packages
    ///
    /// # Security
    ///
    /// The search query is validated before execution to prevent command injection
    pub fn search(&self, query: &str, formula_only: bool, cask_only: bool) -> Result<Vec<String>> {
        // Validate search query
        let validated_query = validation::validate_search_query(query)?.to_string();
        
        let mut args = vec!["search"];
        
        if formula_only {
            args.push("--formula");
        } else if cask_only {
            args.push("--cask");
        }
        
        args.push(&validated_query);
        
        let output = self.core.execute_brew_command(&args)?;
        Ok(self.core.parse_list_output(output))
    }
    
    /// Get detailed information about a formula
    pub fn get_formula_info(&self, formula: &str) -> Result<FormulaInfo> {
        // Validate formula name
        let validated_formula = validation::validate_package_name(formula)?;
        
        let output = self.core.execute_brew_command(&["info", validated_formula])?;
        let info_text = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = info_text.lines().collect();
        
        let mut version = String::new();
        let mut description = String::new();
        
        if !lines.is_empty() {
            // First line usually contains the formula name and version
            let first_line = lines[0];
            if let Some(version_index) = first_line.find(':') {
                version = first_line[version_index+1..].trim().to_string();
            }
            
            // Description is usually in the second line
            if lines.len() > 1 {
                description = lines[1].trim().to_string();
            }
        }
        
        Ok(FormulaInfo {
            name: validated_formula.to_string(),
            version: version.trim().to_string(),
            description: description.trim().to_string(),
        })
    }
    
    /// Get detailed information about a cask
    pub fn get_cask_info(&self, cask: &str) -> Result<CaskInfo> {
        // Validate cask name
        let validated_cask = validation::validate_package_name(cask)?;
        
        let output = self.core.execute_brew_command(&["info", "--cask", validated_cask])?;
        let info_text = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = info_text.lines().collect();
        
        let mut version = String::new();
        let mut description = String::new();
        
        if !lines.is_empty() {
            // Process output to extract version and description
            for line in lines {
                if line.contains("version:") {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() > 1 {
                        version = parts[1].trim().to_string();
                    }
                } else if !line.starts_with(validated_cask) && !line.contains("==>") {
                    description = line.trim().to_string();
                    break;
                }
            }
        }
        
        Ok(CaskInfo {
            name: validated_cask.to_string(),
            version: version.trim().to_string(),
            description: description.trim().to_string(),
        })
    }
    
    /// Search homebrew formulas and display results
    pub fn search_and_display_homebrew(&self, query: &str, deep: bool) -> Result<usize> {
        // Validate query
        let validated_query = validation::validate_search_query(query)?.to_string();
        
        let results = self.search(&validated_query, true, false)?;
        
        if results.is_empty() {
            return Ok(0);
        }
        
        let mut count = 0;
        
        for formula_name in &results {
            count += 1;
            
            // Get additional info if deep search requested
            if deep {
                match self.get_formula_info(formula_name) {
                    Ok(formula_info) => {
                        println!("  {} ({})", style(&formula_info.name).bold(), formula_info.version);
                        if !formula_info.description.is_empty() {
                            println!("    {}", formula_info.description);
                        }
                    },
                    Err(_) => {
                        println!("  {}", formula_name);
                    }
                }
            } else {
                println!("  {}", formula_name);
            }
        }
        
        Ok(count)
    }
    
    /// Search homebrew casks and display results
    pub fn search_and_display_casks(&self, query: &str, deep: bool) -> Result<usize> {
        // Validate query
        let validated_query = validation::validate_search_query(query)?.to_string();
        
        let results = self.search(&validated_query, false, true)?;
        
        if results.is_empty() {
            return Ok(0);
        }
        
        let mut count = 0;
        
        for cask_name in &results {
            count += 1;
            
            // Get additional info if deep search requested
            if deep {
                match self.get_cask_info(cask_name) {
                    Ok(cask_info) => {
                        println!("  {} ({})", style(&cask_info.name).bold(), cask_info.version);
                        if !cask_info.description.is_empty() {
                            println!("    {}", cask_info.description);
                        }
                    },
                    Err(_) => {
                        println!("  {}", cask_name);
                    }
                }
            } else {
                println!("  {}", cask_name);
            }
        }
        
        Ok(count)
    }
    
    /// Search both Homebrew formulas and casks and display results
    pub fn search_and_display_all(&self, query: &str, deep: bool) -> Result<(usize, usize)> {
        // Search formulas
        println!("\n::: 🍺 BREW FORMULAS :::\n");
        let formula_count = self.search_and_display_homebrew(query, deep)?;
        if formula_count == 0 {
            println!("!!!result empty:::");
        }
        
        // Search casks
        println!("\n::: 🍻 BREW CASKS :::\n");
        let cask_count = self.search_and_display_casks(query, deep)?;
        if cask_count == 0 {
            println!("!!!result empty:::");
        }
        
        Ok((formula_count, cask_count))
    }
}

/// Main search function, used by the CLI
pub fn search(query: &str, search_type: &str, deep: bool) -> Result<()> {
    let searcher = BrewSearcher::new();
    let query = query.to_lowercase();
    let search_type = search_type.to_lowercase();
    
    // Determine search type
    match search_type.as_str() {
        "brew" => {
            println!(":::searching homebrew packages for '{}' :::", query);
            match searcher.search_and_display_homebrew(&query, deep) {
                Ok(count) => {
                    if count == 0 {
                        println!("!!!result empty:::");
                    }
                }
                Err(e) => eprintln!("!!!search failed: {}:::", e),
            }
        }
        "cask" => {
            println!(":::searching cask packages for '{}' :::", query);
            match searcher.search_and_display_casks(&query, deep) {
                Ok(count) => {
                    if count == 0 {
                        println!("!!!result empty:::");
                    }
                }
                Err(e) => eprintln!("!!!search failed: {}:::", e),
            }
        }
        "any" | _ => {
            println!(":::searching all package types for '{}' :::", query);
            
            match searcher.search_and_display_all(&query, deep) {
                Ok(_) => {
                    println!("\n:::query executed:::");
                }
                Err(e) => {
                    eprintln!("!!!search failed: {}:::", e);
                }
            }
        }
    }
    
    println!("\n:::command {} end:::", style("search").underlined());
    Ok(())
}

/// Get a default searcher instance
pub fn get_searcher() -> BrewSearcher {
    BrewSearcher::new()
}