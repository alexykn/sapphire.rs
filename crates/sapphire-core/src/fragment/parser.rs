use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::{Context, Result};

/// Fragment type enum
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum FragmentType {
    Dotfiles,
    System,
    Network,
    Custom,
}

impl std::fmt::Display for FragmentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FragmentType::Dotfiles => write!(f, "dotfiles"),
            FragmentType::System => write!(f, "system"),
            FragmentType::Network => write!(f, "network"),
            FragmentType::Custom => write!(f, "custom"),
        }
    }
}

/// Base fragment structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Fragment {
    /// Fragment type
    pub fragment_type: FragmentType,
    
    /// Fragment description
    #[serde(default)]
    pub description: String,
    
    /// Additional fields specific to fragment type
    #[serde(flatten)]
    pub content: serde_yaml::Value,
}

/// Dotfiles fragment content
#[derive(Debug, Serialize, Deserialize)]
pub struct DotfilesFragment {
    #[serde(default)]
    pub files: Vec<FileEntry>,
    
    #[serde(default)]
    pub directories: Vec<DirectoryEntry>,
}

/// File entry in a dotfiles fragment
#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub source: String,
    pub target: String,
    
    #[serde(default)]
    pub backup: bool,
    
    #[serde(default)]
    pub mode: Option<String>,
}

/// Directory entry in a dotfiles fragment
#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryEntry {
    pub source: String,
    pub target: String,
    
    #[serde(default)]
    pub backup: bool,
    
    #[serde(default)]
    pub mode: Option<String>,
}

/// System fragment content
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemFragment {
    #[serde(default)]
    pub preferences: Vec<PreferenceEntry>,
}

/// System preference entry
#[derive(Debug, Serialize, Deserialize)]
pub struct PreferenceEntry {
    pub domain: String,
    pub key: String,
    pub value_type: String,
    pub value: serde_yaml::Value,
}

/// Custom fragment content
#[derive(Debug, Serialize, Deserialize)]
pub struct CustomFragment {
    pub script_path: String,
    
    #[serde(default)]
    pub parameters: serde_yaml::Mapping,
}

impl Fragment {
    /// Load a fragment from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = std::fs::File::open(path.as_ref())
            .with_context(|| format!("Failed to open fragment file: {}", path.as_ref().display()))?;
        
        serde_yaml::from_reader(file)
            .with_context(|| format!("Failed to parse fragment file: {}", path.as_ref().display()))
    }
    
    /// Save a fragment to a file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = std::fs::File::create(path.as_ref())
            .with_context(|| format!("Failed to create fragment file: {}", path.as_ref().display()))?;
        
        serde_yaml::to_writer(file, self)
            .with_context(|| format!("Failed to write fragment file: {}", path.as_ref().display()))
    }
}