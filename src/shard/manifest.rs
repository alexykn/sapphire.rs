use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::{Context, Result};

/// Package manifest for Shard
#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub metadata: Metadata,
    
    #[serde(default)]
    pub formulas: Vec<Formula>,
    
    #[serde(default)]
    pub casks: Vec<Cask>,
    
    #[serde(default)]
    pub taps: Vec<Tap>,

    /// Simplified formula list as strings
    #[serde(default)]
    pub formulae: Vec<String>,

    /// Simplified cask list as strings  
    #[serde(default)]
    pub brews: Vec<String>,
}

/// Metadata for the manifest
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Metadata {
    #[serde(default)]
    pub description: String,
    
    #[serde(default)]
    pub protected: bool,
    
    #[serde(default)]
    pub version: String,
}

/// Homebrew formula
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Formula {
    pub name: String,
    
    #[serde(default = "default_version")]
    pub version: String,
    
    #[serde(default)]
    pub options: Vec<String>,
    
    #[serde(default = "default_state")]
    pub state: PackageState,
}

/// Homebrew cask
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Cask {
    pub name: String,
    
    #[serde(default = "default_version")]
    pub version: String,
    
    #[serde(default)]
    pub options: Vec<String>,
    
    #[serde(default = "default_state")]
    pub state: PackageState,
}

/// Package state (present, absent, latest)
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PackageState {
    Present,
    Absent,
    Latest,
}

/// Homebrew tap
#[derive(Debug, Serialize, Deserialize)]
pub struct Tap {
    pub name: String,
}

fn default_version() -> String {
    "latest".to_string()
}

fn default_state() -> PackageState {
    PackageState::Present
}

impl Manifest {
    /// Create a new empty manifest
    pub fn new() -> Self {
        Self {
            metadata: Metadata {
                description: "Package manifest".to_string(),
                protected: false,
                version: "0.1.0".to_string(),
            },
            formulas: Vec::new(),
            casks: Vec::new(),
            taps: Vec::new(),
            formulae: Vec::new(),
            brews: Vec::new(),
        }
    }
    
    /// Load a manifest from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read manifest file: {}", path.as_ref().display()))?;
        
        // Parse the TOML content into a generic Value first
        let parsed_content = toml::from_str::<toml::Value>(&content)
            .with_context(|| format!("Failed to parse manifest file: {}", path.as_ref().display()))?;
        
        // Create an empty manifest
        let mut manifest = Manifest::new();
        
        // Process metadata if present
        if let Some(metadata) = parsed_content.get("metadata") {
            if let Some(metadata_table) = metadata.as_table() {
                if let Some(description) = metadata_table.get("description").and_then(|v| v.as_str()) {
                    manifest.metadata.description = description.to_string();
                }
                if let Some(protected) = metadata_table.get("protected").and_then(|v| v.as_bool()) {
                    manifest.metadata.protected = protected;
                }
                if let Some(version) = metadata_table.get("version").and_then(|v| v.as_str()) {
                    manifest.metadata.version = version.to_string();
                }
            }
        }
        
        // Process taps - handle both formats
        if let Some(taps_value) = parsed_content.get("taps") {
            // Format 1: Simple array of strings
            if let Some(tap_strings) = taps_value.as_array() {
                for tap in tap_strings {
                    if let Some(tap_str) = tap.as_str() {
                        manifest.taps.push(Tap {
                            name: tap_str.to_string(),
                        });
                    }
                }
            }
        }
        
        // Process formulas - handle both formats
        if let Some(formulas_value) = parsed_content.get("formulas") {
            if let Some(formulas_array) = formulas_value.as_array() {
                for formula in formulas_array {
                    if let Some(formula_table) = formula.as_table() {
                        if let Some(name) = formula_table.get("name").and_then(|v| v.as_str()) {
                            let version = formula_table.get("version")
                                .and_then(|v| v.as_str())
                                .unwrap_or("latest")
                                .to_string();
                            
                            let mut options = Vec::new();
                            if let Some(opts) = formula_table.get("options").and_then(|v| v.as_array()) {
                                for opt in opts {
                                    if let Some(opt_str) = opt.as_str() {
                                        options.push(opt_str.to_string());
                                    }
                                }
                            }
                            
                            let state = match formula_table.get("state").and_then(|v| v.as_str()) {
                                Some("absent") => PackageState::Absent,
                                Some("latest") => PackageState::Latest,
                                _ => PackageState::Present,
                            };
                            
                            manifest.formulas.push(Formula {
                                name: name.to_string(),
                                version,
                                options,
                                state,
                            });
                        }
                    }
                }
            }
        }
        
        // Process casks - handle both formats
        if let Some(casks_value) = parsed_content.get("casks") {
            if let Some(casks_array) = casks_value.as_array() {
                for cask in casks_array {
                    if let Some(cask_table) = cask.as_table() {
                        if let Some(name) = cask_table.get("name").and_then(|v| v.as_str()) {
                            let version = cask_table.get("version")
                                .and_then(|v| v.as_str())
                                .unwrap_or("latest")
                                .to_string();
                            
                            let mut options = Vec::new();
                            if let Some(opts) = cask_table.get("options").and_then(|v| v.as_array()) {
                                for opt in opts {
                                    if let Some(opt_str) = opt.as_str() {
                                        options.push(opt_str.to_string());
                                    }
                                }
                            }
                            
                            let state = match cask_table.get("state").and_then(|v| v.as_str()) {
                                Some("absent") => PackageState::Absent,
                                Some("latest") => PackageState::Latest,
                                _ => PackageState::Present,
                            };
                            
                            manifest.casks.push(Cask {
                                name: name.to_string(),
                                version,
                                options,
                                state,
                            });
                        }
                    }
                }
            }
        }
        
        // Process simplified formulae list
        if let Some(formulae_value) = parsed_content.get("formulae") {
            if let Some(formulae_array) = formulae_value.as_array() {
                for formula in formulae_array {
                    if let Some(formula_str) = formula.as_str() {
                        // Add to the manifest.formulae collection
                        manifest.formulae.push(formula_str.to_string());
                        
                        // Also process into the structured format
                        if let Some((name, version)) = formula_str.split_once(':') {
                            manifest.formulas.push(Formula {
                                name: name.trim().to_string(),
                                version: version.trim().to_string(),
                                options: Vec::new(),
                                state: PackageState::Present,
                            });
                        } else {
                            manifest.formulas.push(Formula {
                                name: formula_str.trim().to_string(),
                                version: "latest".to_string(),
                                options: Vec::new(),
                                state: PackageState::Present,
                            });
                        }
                    }
                }
            }
        }
        
        // Process simplified brews list
        if let Some(brews_value) = parsed_content.get("brews") {
            if let Some(brews_array) = brews_value.as_array() {
                for brew in brews_array {
                    if let Some(brew_str) = brew.as_str() {
                        // Add to the manifest.brews collection
                        manifest.brews.push(brew_str.to_string());
                        
                        // Also process into the structured format
                        if let Some((name, version)) = brew_str.split_once(':') {
                            manifest.casks.push(Cask {
                                name: name.trim().to_string(),
                                version: version.trim().to_string(),
                                options: Vec::new(),
                                state: PackageState::Present,
                            });
                        } else {
                            manifest.casks.push(Cask {
                                name: brew_str.trim().to_string(),
                                version: "latest".to_string(),
                                options: Vec::new(),
                                state: PackageState::Present,
                            });
                        }
                    }
                }
            }
        }
        
        Ok(manifest)
    }
    
    /// Save a manifest to a file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        // Create a clean compact manifest for export
        let mut clean_manifest = toml::value::Table::new();
        
        // Add metadata at the top
        let mut metadata_table = toml::value::Table::new();
        metadata_table.insert("description".to_string(), toml::Value::String(self.metadata.description.clone()));
        metadata_table.insert("protected".to_string(), toml::Value::Boolean(self.metadata.protected));
        metadata_table.insert("version".to_string(), toml::Value::String(self.metadata.version.clone()));
        clean_manifest.insert("metadata".to_string(), toml::Value::Table(metadata_table));
        
        // Add taps as simple strings
        let mut taps = Vec::new();
        for tap in &self.taps {
            taps.push(toml::Value::String(tap.name.clone()));
        }
        if !taps.is_empty() {
            clean_manifest.insert("taps".to_string(), toml::Value::Array(taps));
        }
        
        // Add formulae as simple strings, tracking to avoid duplicates
        let mut formulae = Vec::new();
        let mut formula_names = std::collections::HashSet::new();
        
        // Convert detailed formulas to simplified list
        for formula in &self.formulas {
            if formula.state == PackageState::Present || formula.state == PackageState::Latest {
                formula_names.insert(formula.name.clone());
                
                if formula.version == "latest" {
                    formulae.push(toml::Value::String(formula.name.clone()));
                } else {
                    formulae.push(toml::Value::String(format!("{}:{}", formula.name, formula.version)));
                }
            }
        }
        
        // Also include directly set formulae if not empty and not duplicate
        for formula in &self.formulae {
            // Extract name part in case it has a version like "name:version"
            let name = if let Some(pos) = formula.find(':') {
                &formula[..pos]
            } else {
                formula
            };
            
            if !formula_names.contains(name) {
                formula_names.insert(name.to_string());
                formulae.push(toml::Value::String(formula.clone()));
            }
        }
        
        if !formulae.is_empty() {
            clean_manifest.insert("formulae".to_string(), toml::Value::Array(formulae));
        }
        
        // Add brews (casks) as simple strings, tracking to avoid duplicates
        let mut brews = Vec::new();
        let mut brew_names = std::collections::HashSet::new();
        
        // Convert detailed casks to simplified list
        for cask in &self.casks {
            if cask.state == PackageState::Present || cask.state == PackageState::Latest {
                brew_names.insert(cask.name.clone());
                
                if cask.version == "latest" {
                    brews.push(toml::Value::String(cask.name.clone()));
                } else {
                    brews.push(toml::Value::String(format!("{}:{}", cask.name, cask.version)));
                }
            }
        }
        
        // Also include directly set brews if not empty and not duplicate
        for brew in &self.brews {
            // Extract name part in case it has a version like "name:version"
            let name = if let Some(pos) = brew.find(':') {
                &brew[..pos]
            } else {
                brew
            };
            
            if !brew_names.contains(name) {
                brew_names.insert(name.to_string());
                brews.push(toml::Value::String(brew.clone()));
            }
        }
        
        if !brews.is_empty() {
            clean_manifest.insert("brews".to_string(), toml::Value::Array(brews));
        }
        
        // Generate TOML content
        let toml_content = toml::to_string_pretty(&clean_manifest)
            .with_context(|| format!("Failed to serialize manifest to TOML: {}", path.as_ref().display()))?;
        
        // Write to file
        std::fs::write(path.as_ref(), toml_content)
            .with_context(|| format!("Failed to write manifest file: {}", path.as_ref().display()))?;
        
        Ok(())
    }
}

impl Clone for Manifest {
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            formulas: self.formulas.clone(),
            casks: self.casks.clone(),
            taps: self.taps.clone(),
            formulae: Vec::new(), // Don't copy these as they'll be regenerated
            brews: Vec::new(),    // Don't copy these as they'll be regenerated
        }
    }
}

impl Clone for Metadata {
    fn clone(&self) -> Self {
        Self {
            description: self.description.clone(),
            protected: self.protected,
            version: self.version.clone(),
        }
    }
}

impl Clone for Tap {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
        }
    }
}
