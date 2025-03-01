use serde::{Deserialize, Serialize};
use std::path::Path;

use super::error::{SapphireError, SapphireResult};

/// Represents the main Sapphire configuration 
#[derive(Debug, Serialize, Deserialize)]
pub struct SapphireConfig {
    #[serde(default)]
    pub version: String,
    
    #[serde(default)]
    pub system: SystemConfig,
    
    #[serde(default)]
    pub packages: Vec<Package>,
    
    #[serde(default)]
    pub fragments: Vec<String>,
}

impl Default for SapphireConfig {
    fn default() -> Self {
        Self {
            version: "0.1.0".to_string(),
            system: SystemConfig::default(),
            packages: Vec::new(),
            fragments: Vec::new(),
        }
    }
}

/// System-level configuration
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SystemConfig {
    #[serde(default)]
    pub hostname: Option<String>,
    
    #[serde(default)]
    pub preferences: Vec<SystemPreference>,
}

/// System preference setting
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemPreference {
    pub domain: String,
    pub key: String,
    pub value: PreferenceValue,
}

/// Represents different types of values for system preferences
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreferenceValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

/// Package representation for the package manager
#[derive(Debug, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub state: PackageState,
    #[serde(default)]
    pub options: Vec<String>,
}

/// Package installation state
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageState {
    Present,
    Absent,
    Latest,
}

impl SapphireConfig {
    /// Load configuration from a YAML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> SapphireResult<Self> {
        let file = std::fs::File::open(path)
            .map_err(|e| SapphireError::Io(e))?;
        
        serde_yaml::from_reader(file)
            .map_err(|e| SapphireError::YamlParse(e))
    }
    
    /// Save configuration to a YAML file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> SapphireResult<()> {
        let file = std::fs::File::create(path)
            .map_err(|e| SapphireError::Io(e))?;
        
        serde_yaml::to_writer(file, self)
            .map_err(|e| SapphireError::YamlParse(e))
    }
    
    /// Create a new default configuration
    pub fn new() -> Self {
        Self::default()
    }
}