use anyhow::{Context, Result};
use std::process::Command;
use console::style;

/// Search for packages
pub fn search(query: &str, search_type: &str, deep: bool) -> Result<()> {
    let query = query.to_lowercase();
    let search_type = search_type.to_lowercase();
    
    // Determine search type
    match search_type.as_str() {
        "brew" => {
            println!(":::searching homebrew packages for '{}' :::", query);
            match search_homebrew(&query, deep) {
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
            match search_casks(&query, deep) {
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
            
            // Use separate searches for formulas and casks
            match search_brew_all(&query, deep) {
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

/// Search Homebrew formulas
fn search_homebrew(query: &str, deep: bool) -> Result<usize> {
    let output = Command::new("brew")
        .args(["search", "--formula", query])
        .output()
        .context("Failed to execute brew search --formula command")?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Homebrew search failed: {}", error);
    }
    
    let results = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = results.lines().collect();
    
    if lines.is_empty() {
        return Ok(0);
    }
    
    let mut count = 0;
    
    for line in lines {
        // Skip warning lines and section headers
        if line.starts_with("Warning:") || line.is_empty() || 
           line.starts_with("==>") || line.contains("If you meant") {
            continue;
        }
        
        // Extract the formula name
        let formula_name = line.trim();
        
        count += 1;
        
        // Get additional info if deep search requested
        if deep {
            let formula_info = get_formula_info(formula_name)?;
            println!("  {} ({})", style(formula_name).bold(), formula_info.version);
            if !formula_info.description.is_empty() {
                println!("    {}", formula_info.description);
            }
        } else {
            println!("  {}", formula_name);
        }
    }
    
    Ok(count)
}

/// Search Homebrew casks
fn search_casks(query: &str, deep: bool) -> Result<usize> {
    let output = Command::new("brew")
        .args(["search", "--cask", query])
        .output()
        .context("Failed to execute brew search --cask command")?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Cask search failed: {}", error);
    }
    
    let results = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = results.lines().collect();
    
    if lines.is_empty() {
        return Ok(0);
    }
    
    let mut count = 0;
    
    for line in lines {
        // Skip warning lines and section headers
        if line.starts_with("Warning:") || line.is_empty() || 
           line.starts_with("==>") || line.contains("If you meant") {
            continue;
        }
        
        // Extract the cask name
        let cask_name = line.trim();
        
        count += 1;
        
        // Get additional info if deep search requested
        if deep {
            let cask_info = get_cask_info(cask_name)?;
            println!("  {} ({})", style(cask_name).bold(), cask_info.version);
            if !cask_info.description.is_empty() {
                println!("    {}", cask_info.description);
            }
        } else {
            println!("  {}", cask_name);
        }
    }
    
    Ok(count)
}

/// Search both Homebrew formulas and casks
fn search_brew_all(query: &str, deep: bool) -> Result<(usize, usize)> {
    // Rather than trying to parse the mixed output, let's do separate searches for formulas and casks
    
    // Search formulas
    println!("\n::: 🍺 BREW FORMULAS :::\n");
    let formula_count = search_homebrew(query, deep)?;
    if formula_count == 0 {
        println!("!!!result empty:::");
    }
    
    // Search casks
    println!("\n::: 🍻 BREW CASKS :::\n");
    let cask_count = search_casks(query, deep)?;
    if cask_count == 0 {
        println!("!!!result empty:::");
    }
    
    Ok((formula_count, cask_count))
}

/// Formula information structure
struct FormulaInfo {
    version: String,
    description: String,
}

/// Get detailed information about a Homebrew formula
fn get_formula_info(formula: &str) -> Result<FormulaInfo> {
    let output = Command::new("brew")
        .arg("info")
        .arg(formula)
        .output()
        .context(format!("Failed to get info for formula: {}", formula))?;
    
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
        version: version.trim().to_string(),
        description: description.trim().to_string(),
    })
}

/// Cask information structure
struct CaskInfo {
    version: String,
    description: String,
}

/// Get detailed information about a Homebrew cask
fn get_cask_info(cask: &str) -> Result<CaskInfo> {
    let output = Command::new("brew")
        .arg("info")
        .arg("--cask")
        .arg(cask)
        .output()
        .context(format!("Failed to get info for cask: {}", cask))?;
    
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
            } else if !line.starts_with(cask) && !line.contains("==>") {
                description = line.trim().to_string();
                break;
            }
        }
    }
    
    Ok(CaskInfo {
        version: version.trim().to_string(),
        description: description.trim().to_string(),
    })
}