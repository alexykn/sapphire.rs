use serde::{Deserialize, Serialize};
use crate::utils::ShardResult;
use std::path::Path;
use anyhow::Context;
use crate::utils::filesystem;
use crate::utils::log_debug;

/// Package manifest for Shard
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Manifest {
    /// Array of formula names - preferred over structured formulas
    #[serde(default)]
    pub formulae: Vec<String>,
    
    /// Array of cask names - preferred over structured casks
    #[serde(default)]
    pub casks: Vec<String>,
    
    /// Array of tap names - preferred over structured taps
    #[serde(default)]
    pub taps: Vec<String>,
    
    /// Legacy structured formulas representation
    #[serde(default, skip_serializing)]
    pub formulas: Vec<Formula>,
    
    /// Legacy structured casks representation
    #[serde(default, skip_serializing)]
    pub casks_structured: Vec<Cask>,
    
    /// Legacy structured taps representation
    #[serde(default, skip_serializing)]
    pub taps_structured: Vec<Tap>,
    
    #[serde(default)]
    pub metadata: Metadata,
}

/// Metadata for the manifest
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Metadata {
    /// Name of the shard
    #[serde(default)]
    pub name: String,
    
    /// Brief description of the shard
    #[serde(default)]
    pub description: String,
    
    /// Owner of the shard
    #[serde(default)]
    pub owner: String,
    
    /// Whether this shard is protected from modifications
    #[serde(default)]
    pub protected: bool,
    
    /// Shard schema version
    #[serde(default)]
    pub version: String,
    
    /// List of users allowed to modify this shard even if protected
    #[serde(default)]
    pub allowed_users: Vec<String>,
    
    /// DEPRECATED: Protection level (use 'protected' boolean instead)
    #[serde(default, skip_serializing)]
    pub protection_level: u8,
}

/// Package state (present, absent, latest) - kept for compatibility
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PackageState {
    Present,
    Absent,
    Latest,
}

/// Homebrew formula - legacy format
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

/// Homebrew cask - legacy format
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

/// Homebrew tap - legacy format
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tap {
    pub name: String,
}

fn default_version() -> String {
    "latest".to_string()
}

fn default_state() -> PackageState {
    PackageState::Latest
}

impl Manifest {
    /// Create a new empty manifest
    pub fn new() -> Self {
        Self {
            metadata: Metadata {
                name: String::new(),
                description: "Package manifest".to_string(),
                owner: String::new(),
                protected: false,
                version: "0.1.0".to_string(),
                allowed_users: Vec::new(),
                protection_level: 0,
            },
            formulae: Vec::new(),
            casks: Vec::new(),
            taps: Vec::new(),
            formulas: Vec::new(),
            casks_structured: Vec::new(),
            taps_structured: Vec::new(),
        }
    }
    
    /// Check if a user is allowed to modify this manifest
    pub fn can_modify(&self, _username: &str) -> bool {
        // If not protected, anyone can modify
        !self.metadata.protected
    }
    
    /// Load a manifest from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> ShardResult<Self> {
        log_debug(&format!("Loading manifest from: {}", path.as_ref().display()));
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read manifest file: {}", path.as_ref().display()))?;
        
        // Parse the TOML content
        let mut parsed: Manifest = toml::from_str(&content)
            .with_context(|| format!("Failed to parse manifest file: {}", path.as_ref().display()))?;
            
        // Handle migration between the different formats
        
        // 1. Check if we have structured formulas but no simple formulae array
        if !parsed.formulas.is_empty() && parsed.formulae.is_empty() {
            // Migrate to simple format
            for formula in &parsed.formulas {
                if !parsed.formulae.contains(&formula.name) {
                    parsed.formulae.push(formula.name.clone());
                }
            }
        }
        
        // 2. Check if we have structured casks but no simple casks array
        if !parsed.casks_structured.is_empty() && parsed.casks.is_empty() {
            // Migrate to simple format
            for cask in &parsed.casks_structured {
                if !parsed.casks.contains(&cask.name) {
                    parsed.casks.push(cask.name.clone());
                }
            }
        }
        
        // 3. Check if we have structured taps but no simple taps array
        if !parsed.taps_structured.is_empty() && parsed.taps.is_empty() {
            // Migrate to simple format
            for tap in &parsed.taps_structured {
                if !parsed.taps.contains(&tap.name) {
                    parsed.taps.push(tap.name.clone());
                }
            }
        }
        
        // 4. Migrate from 'brews' field if it exists in the raw TOML (backward compatibility)
        if let Ok(raw_value) = toml::from_str::<toml::Value>(&content) {
            // Process legacy 'brews' field for casks
            if let Some(brews) = raw_value.get("brews").and_then(|v| v.as_array()) {
                for brew in brews {
                    if let Some(name) = brew.as_str() {
                        let name_string = name.to_string();
                        if !parsed.casks.contains(&name_string) {
                            parsed.casks.push(name_string);
                        }
                    }
                }
            }
        }
        
        Ok(parsed)
    }
    
    /// Save a manifest to a file - outputs simplified format
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> ShardResult<()> {
        log_debug(&format!("Saving manifest to: {}", path.as_ref().display()));
        
        // Create a simplified representation for serialization
        let simplified = SimplifiedManifest {
            formulae: self.formulae.clone(),
            casks: self.casks.clone(),
            taps: self.taps.clone(),
            metadata: self.metadata.clone(),
        };
        
        // Serialize to TOML
        let toml_content = toml::to_string_pretty(&simplified)
            .with_context(|| "Failed to serialize manifest to TOML")?;
        
        // Ensure parent directory exists
        filesystem::ensure_parent_dir_exists(path.as_ref())?;
        
        // Write to file
        std::fs::write(path.as_ref(), &toml_content)
            .with_context(|| format!("Failed to write manifest to file: {}", path.as_ref().display()))?;
        
        Ok(())
    }
    
    /// Update modification info - placeholder for future use
    pub fn update_modification_info(&mut self) {
        // Currently a placeholder, will be updated with user/timestamp later
    }
    
    /// Check if the manifest is protected
    pub fn is_protected(&self) -> bool {
        self.metadata.protected
    }
}

/// Simplified manifest structure for serialization
#[derive(Serialize)]
struct SimplifiedManifest {
    formulae: Vec<String>,
    casks: Vec<String>,
    taps: Vec<String>,
    metadata: Metadata,
}
