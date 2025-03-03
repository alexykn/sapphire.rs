#[allow(dead_code)]
use anyhow::{Context, Result, bail};
use console::style;
use dialoguer::Confirm;
use shellexpand;
use std::fs;
use std::io::{self};
use std::path::PathBuf;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fmt;
use std::error::Error as StdError;
use crate::core::manifest::Manifest;

/// Custom error type for shard management operations
#[derive(Debug)]
pub enum ShardError {
    /// Error when a shard doesn't exist
    NotFound(String),
    /// Error when a shard name is invalid
    InvalidName(String),
    /// Error when a shard already exists
    AlreadyExists(String),
    /// Error when trying to modify a protected shard
    Protected(String),
    /// Error with filesystem operations
    Filesystem { path: PathBuf, source: io::Error },
    /// Error parsing a manifest
    ManifestError(String),
    /// Error with a backup operation
    BackupError { name: String, source: Box<dyn StdError + Send + Sync> },
    /// Other errors
    Other(anyhow::Error),
}

impl fmt::Display for ShardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(name) => write!(f, "Shard '{}' not found", name),
            Self::InvalidName(name) => write!(f, "Invalid shard name: {}", name),
            Self::AlreadyExists(name) => write!(f, "Shard '{}' already exists", name),
            Self::Protected(name) => write!(f, "Cannot modify protected shard: {}", name),
            Self::Filesystem { path, source } => write!(f, "Filesystem error at {}: {}", path.display(), source),
            Self::ManifestError(msg) => write!(f, "Manifest error: {}", msg),
            Self::BackupError { name, source } => write!(f, "Backup error for shard '{}': {}", name, source),
            Self::Other(e) => write!(f, "Error: {}", e),
        }
    }
}

impl StdError for ShardError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Filesystem { source, .. } => Some(source),
            Self::BackupError { source, .. } => Some(source.as_ref()),
            Self::Other(e) => e.source(),
            _ => None,
        }
    }
}

impl From<anyhow::Error> for ShardError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(err)
    }
}

impl From<io::Error> for ShardError {
    fn from(err: io::Error) -> Self {
        Self::Other(err.into())
    }
}

/// Result type with ShardError
pub type ShardResult<T> = std::result::Result<T, ShardError>;

/// Status of a shard
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShardStatus {
    /// Shard is active
    Active,
    /// Shard is disabled
    Disabled,
    /// Shard is not found
    NotFound,
}

/// Information about a shard
#[derive(Debug, Clone)]
pub struct ShardInfo {
    /// Name of the shard
    pub name: String,
    /// Path to the shard file
    pub path: PathBuf,
    /// Current status of the shard
    pub status: ShardStatus,
    /// Shard manifest if available
    pub manifest: Option<Manifest>,
}

/// Manager for shard operations
pub struct ShardManager {
    /// Directory where active shards are stored
    shards_dir: PathBuf,
    /// Directory where disabled shards are stored
    disabled_dir: PathBuf,
    /// Directory where backups are stored
    backups_dir: PathBuf,
    /// Protected shard names that cannot be disabled
    protected_shards: Vec<String>,
    /// Current username for permission checks
    current_user: String,
}

impl ShardManager {
    /// Create a new shard manager with default paths
    pub fn new() -> Result<Self> {
        // Expand default paths
        let shards_dir = shellexpand::tilde("~/.sapphire/shards").to_string();
        let disabled_dir = format!("{}/disabled", shards_dir);
        let backups_dir = shellexpand::tilde("~/.sapphire/backups").to_string();
        
        // Get current username for permission checks
        let current_user = match std::env::var("USER") {
            Ok(user) => user,
            Err(_) => "unknown".to_string(),
        };
        
        Ok(Self {
            shards_dir: PathBuf::from(shards_dir),
            disabled_dir: PathBuf::from(disabled_dir),
            backups_dir: PathBuf::from(backups_dir),
            protected_shards: vec!["system".to_string(), "user".to_string()],
            current_user,
        })
    }
    
    /// Create a new shard manager with custom paths
    pub fn with_paths(shards_dir: PathBuf, disabled_dir: PathBuf) -> Self {
        let backups_dir = shellexpand::tilde("~/.sapphire/backups").to_string();
        
        // Get current username for permission checks
        let current_user = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
        
        Self {
            shards_dir,
            disabled_dir,
            backups_dir: PathBuf::from(backups_dir),
            protected_shards: vec!["system".to_string(), "user".to_string()],
            current_user,
        }
    }
    
    /// Create a new shard manager with custom paths including backups
    pub fn with_all_paths(shards_dir: PathBuf, disabled_dir: PathBuf, backups_dir: PathBuf) -> Self {
        // Get current username for permission checks
        let current_user = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
        
        Self {
            shards_dir,
            disabled_dir,
            backups_dir,
            protected_shards: vec!["system".to_string(), "user".to_string()],
            current_user,
        }
    }
    
    /// Set the current user for permission checks
    pub fn with_user(mut self, username: &str) -> Self {
        self.current_user = username.to_string();
        self
    }
    
    /// Set protected shards that cannot be disabled
    pub fn with_protected_shards(mut self, shards: Vec<String>) -> Self {
        self.protected_shards = shards;
        self
    }
    
    /// Check if a shard is protected by name or manifest
    fn is_protected(&self, name: &str) -> Result<bool> {
        // First check by name in our protected list
        if self.protected_shards.contains(&name.to_string()) {
            return Ok(true);
        }
        
        // Then check manifest's protected flag if available
        let shard_path = self.get_shard_path(name);
        if shard_path.exists() {
            if let Ok(manifest) = Manifest::from_file(shard_path.to_str().unwrap_or_default()) {
                // Use the new protection level field
                let is_protected = manifest.metadata.protected || manifest.metadata.protection_level > 0;
                return Ok(is_protected);
            }
        }
        
        // Check disabled path as well
        let disabled_path = self.get_disabled_shard_path(name);
        if disabled_path.exists() {
            if let Ok(manifest) = Manifest::from_file(disabled_path.to_str().unwrap_or_default()) {
                // Use the new protection level field
                let is_protected = manifest.metadata.protected || manifest.metadata.protection_level > 0;
                return Ok(is_protected);
            }
        }
        
        Ok(false)
    }
    
    /// Check if the current user can modify a shard
    fn can_modify_shard(&self, name: &str) -> Result<bool> {
        let shard_path = self.get_shard_path(name);
        if shard_path.exists() {
            if let Ok(manifest) = Manifest::from_file(shard_path.to_str().unwrap_or_default()) {
                return Ok(manifest.can_modify(&self.current_user));
            }
        }
        
        // Check disabled path as well
        let disabled_path = self.get_disabled_shard_path(name);
        if disabled_path.exists() {
            if let Ok(manifest) = Manifest::from_file(disabled_path.to_str().unwrap_or_default()) {
                return Ok(manifest.can_modify(&self.current_user));
            }
        }
        
        // Default to true - if shard doesn't exist yet, we can create it
        Ok(true)
    }
    
    /// Create a backup of a shard before modification
    fn backup_shard(&self, name: &str) -> Result<PathBuf> {
        let shard_path = self.get_shard_path(name);
        if !shard_path.exists() {
            bail!("Cannot backup non-existent shard: {}", name);
        }
        
        // Create backup directory if it doesn't exist
        fs::create_dir_all(&self.backups_dir)
            .with_context(|| format!("Failed to create backup directory: {}", self.backups_dir.display()))?;
            
        // Generate a timestamp for the backup filename
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let backup_path = self.backups_dir.join(format!("{}_backup_{}.toml", name, timestamp));
        
        // Copy the shard file to the backup location
        fs::copy(&shard_path, &backup_path)
            .with_context(|| format!("Failed to backup shard {} to {}", name, backup_path.display()))?;
        
        Ok(backup_path)
    }
    
    /// Create a new shard
    pub fn grow_shard(&self, name: &str, description: Option<&str>) -> Result<()> {
        println!("{} new shard: {}", style("Growing").bold().green(), style(name).bold());
        
        // Validate shard name
        if !self.is_valid_shard_name(name) {
            bail!("Invalid shard name. Names must only contain letters, numbers, underscores, and hyphens");
        }
        
        // Check if shard already exists
        let shard_path = self.get_shard_path(name);
        
        if shard_path.exists() {
            bail!("A shard with name '{}' already exists", name);
        }
        
        // Create shards directory if it doesn't exist
        fs::create_dir_all(&self.shards_dir)
            .context("Failed to create shards directory")?;
        
        // Create a new empty manifest
        let mut manifest = Manifest::new();
        manifest.metadata.description = description
            .unwrap_or(&format!("Custom shard: {}", name))
            .to_string();
        
        // Auto-allow current user to modify
        if !self.current_user.is_empty() && self.current_user != "unknown" {
            manifest.metadata.allowed_users.push(self.current_user.clone());
        }
        
        // Set last modified information
        manifest.update_modification_info();
        
        // Save the manifest
        manifest.to_file(shard_path.to_str().unwrap_or_default())
            .context("Failed to create shard file")?;
        
        println!("{} Created new shard: {}", style("✓").bold().green(), style(&shard_path.display()).bold());
        Ok(())
    }
    
    /// Delete a shard
    pub fn shatter_shard(&self, name: &str, force: bool) -> Result<()> {
        // Validate shard name for safety
        if !self.is_valid_shard_name(name) {
            bail!("Invalid shard name. Names must only contain letters, numbers, underscores, and hyphens");
        }
        
        // Check if it's a protected shard
        if self.is_protected(name)? {
            // Additional check if the user can modify despite it being protected
            if !self.can_modify_shard(name)? {
                bail!("Cannot delete protected shard: {}", name);
            } else if !force {
                println!("{} Shard '{}' is protected but you have permission to delete it", 
                    style("Warning:").bold().yellow(), style(name).bold());
                if !Confirm::new().with_prompt("Are you sure you want to delete this protected shard?").default(false).interact()? {
                    println!("Operation cancelled");
                    return Ok(());
                }
            }
        }
        
        // Get shard path
        let shard_path = self.get_shard_path(name);
        let disabled_path = self.get_disabled_shard_path(name);
        
        // Check if shard exists (either enabled or disabled)
        let is_active = shard_path.exists();
        let is_disabled = disabled_path.exists();
        
        if !is_active && !is_disabled {
            bail!("Shard '{}' not found", name);
        }
        
        // Create backup for audit trail before deletion
        if is_active {
            match self.backup_shard(name) {
                Ok(backup_path) => {
                    println!("Created backup at: {}", backup_path.display());
                },
                Err(e) => {
                    println!("Warning: Failed to create backup before deletion: {}", e);
                    if !force {
                        bail!("Aborting deletion due to backup failure");
                    }
                }
            }
        }
        
        // Confirm deletion unless force is used
        if !force {
            let status = if is_disabled { "disabled" } else { "active" };
            println!("About to delete {} shard: {}", status, style(name).bold());
            if !Confirm::new().with_prompt("Are you sure?").default(false).interact()? {
                println!("Operation cancelled");
                return Ok(());
            }
        }
        
        // Delete the file
        if is_active {
            fs::remove_file(&shard_path)
                .with_context(|| format!("Failed to delete shard: {}", name))?;
        } else if is_disabled {
            fs::remove_file(&disabled_path)
                .with_context(|| format!("Failed to delete disabled shard: {}", name))?;
        }
        
        println!("{} Deleted shard: {}", style("✓").bold().green(), style(name).bold());
        Ok(())
    }
    
    /// Disable a shard without deleting it
    pub fn disable_shard(&self, name: &str) -> Result<()> {
        // Validate shard name for safety
        if !self.is_valid_shard_name(name) {
            bail!("Invalid shard name. Names must only contain letters, numbers, underscores, and hyphens");
        }
        
        // Check if it's a protected shard
        if self.is_protected(name)? {
            // Additional check if the user can modify despite it being protected
            if !self.can_modify_shard(name)? {
                bail!("Cannot disable protected shard: {}", name);
            } else {
                println!("{} Shard '{}' is protected but you have permission to disable it", 
                    style("Warning:").bold().yellow(), style(name).bold());
                if !Confirm::new().with_prompt("Are you sure you want to disable this protected shard?").default(false).interact()? {
                    println!("Operation cancelled");
                    return Ok(());
                }
            }
        }
        
        // Get source and destination paths
        let source_path = self.get_shard_path(name);
        
        // Check if shard exists
        if !source_path.exists() {
            // Check if it's already disabled
            if self.get_disabled_shard_path(name).exists() {
                println!("{} Shard '{}' is already disabled", style("!").bold().yellow(), style(name).bold());
                return Ok(());
            }
            
            bail!("Shard '{}' not found", name);
        }
        
        // Create backup before disabling
        match self.backup_shard(name) {
            Ok(backup_path) => {
                println!("Created backup at: {}", backup_path.display());
            },
            Err(e) => {
                println!("Warning: Failed to create backup before disabling: {}", e);
            }
        }
        
        // Create disabled directory if it doesn't exist
        fs::create_dir_all(&self.disabled_dir)
            .context("Failed to create disabled shards directory")?;
        
        let dest_path = self.get_disabled_shard_path(name);
        
        // Move the file to disabled directory
        fs::rename(&source_path, &dest_path)
            .with_context(|| format!("Failed to disable shard: {}", name))?;
        
        println!("{} Disabled shard: {}", style("✓").bold().green(), style(name).bold());
        Ok(())
    }
    
    /// Enable a previously disabled shard
    pub fn enable_shard(&self, name: &str) -> Result<()> {
        // Validate shard name for safety
        if !self.is_valid_shard_name(name) {
            bail!("Invalid shard name. Names must only contain letters, numbers, underscores, and hyphens");
        }
        
        // Get source and destination paths
        let source_path = self.get_disabled_shard_path(name);
        
        // Check if disabled shard exists
        if !source_path.exists() {
            // Check if it's already enabled
            let enabled_path = self.get_shard_path(name);
            if enabled_path.exists() {
                println!("{} Shard '{}' is already enabled", style("!").bold().yellow(), style(name).bold());
                return Ok(());
            }
            
            bail!("Shard '{}' not found in active or disabled shards", name);
        }
        
        // Create shards directory if it doesn't exist
        fs::create_dir_all(&self.shards_dir)
            .context("Failed to create shards directory")?;
        
        let dest_path = self.get_shard_path(name);
        
        // Read the manifest to update last modified information
        if let Ok(mut manifest) = Manifest::from_file(source_path.to_str().unwrap_or_default()) {
            // Update modification info
            manifest.update_modification_info();
            
            // Write the updated manifest directly to the destination
            manifest.to_file(dest_path.to_str().unwrap_or_default())?;
            
            // Delete the source file
            fs::remove_file(&source_path)?;
        } else {
            // Fall back to simple file move if manifest can't be read
            fs::rename(&source_path, &dest_path)
                .with_context(|| format!("Failed to enable shard: {}", name))?;
        }
        
        println!("{} Enabled shard: {}", style("✓").bold().green(), style(name).bold());
        Ok(())
    }
    
    /// Get the status of a shard
    pub fn get_shard_status(&self, name: &str) -> ShardStatus {
        if !self.is_valid_shard_name(name) {
            return ShardStatus::NotFound;
        }
        
        if self.get_shard_path(name).exists() {
            ShardStatus::Active
        } else if self.get_disabled_shard_path(name).exists() {
            ShardStatus::Disabled
        } else {
            ShardStatus::NotFound
        }
    }
    
    /// Get detailed information about a shard
    pub fn get_shard_info(&self, name: &str) -> Result<ShardInfo> {
        let status = self.get_shard_status(name);
        let path = match status {
            ShardStatus::Active => self.get_shard_path(name),
            ShardStatus::Disabled => self.get_disabled_shard_path(name),
            ShardStatus::NotFound => bail!("Shard '{}' not found", name),
        };
        
        let manifest = if path.exists() {
            match Manifest::from_file(path.to_str().unwrap_or_default()) {
                Ok(manifest) => Some(manifest),
                Err(_) => None,
            }
        } else {
            None
        };
        
        Ok(ShardInfo {
            name: name.to_string(),
            path,
            status,
            manifest,
        })
    }
    
    /// Get information about all shards
    pub fn get_all_shards_info(&self) -> Result<HashMap<String, ShardInfo>> {
        let mut result = HashMap::new();
        
        // Get active shards
        for name in self.list_shards()? {
            if let Ok(info) = self.get_shard_info(&name) {
                result.insert(name, info);
            }
        }
        
        // Get disabled shards
        for name in self.list_disabled_shards()? {
            if let Ok(info) = self.get_shard_info(&name) {
                result.insert(name, info);
            }
        }
        
        Ok(result)
    }
    
    /// Check if a shard exists
    pub fn shard_exists(&self, name: &str) -> bool {
        if !self.is_valid_shard_name(name) {
            return false;
        }
        
        self.get_shard_path(name).exists() || self.get_disabled_shard_path(name).exists()
    }
    
    /// Check if a shard is disabled
    pub fn shard_is_disabled(&self, name: &str) -> bool {
        if !self.is_valid_shard_name(name) {
            return false;
        }
        
        self.get_disabled_shard_path(name).exists()
    }
    
    /// Check if a shard is active (enabled)
    pub fn shard_is_active(&self, name: &str) -> bool {
        if !self.is_valid_shard_name(name) {
            return false;
        }
        
        self.get_shard_path(name).exists()
    }
    
    /// Check if a shard is protected
    pub fn shard_is_protected(&self, name: &str) -> bool {
        self.is_protected(name).unwrap_or(false)
    }
    
    /// Check if a user can modify a shard
    pub fn shard_can_be_modified_by(&self, name: &str, username: &str) -> bool {
        // Create a temporary manager with the specified user
        let temp_manager = self.clone().with_user(username);
        temp_manager.can_modify_shard(name).unwrap_or(false)
    }
    
    /// Clone the ShardManager
    pub fn clone(&self) -> Self {
        Self {
            shards_dir: self.shards_dir.clone(),
            disabled_dir: self.disabled_dir.clone(),
            backups_dir: self.backups_dir.clone(),
            protected_shards: self.protected_shards.clone(),
            current_user: self.current_user.clone(),
        }
    }
    
    /// List all available shards
    pub fn list_shards(&self) -> Result<Vec<String>> {
        // Create directory if it doesn't exist
        if !self.shards_dir.exists() {
            return Ok(Vec::new());
        }
        
        let entries = fs::read_dir(&self.shards_dir)
            .context("Failed to read shards directory")?;
            
        let mut shards = Vec::new();
        
        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();
            
            // Only include .toml files and skip directories
            if path.is_file() && path.extension().map_or(false, |ext| ext == "toml") {
                if let Some(name) = path.file_stem() {
                    if let Some(name_str) = name.to_str() {
                        shards.push(name_str.to_string());
                    }
                }
            }
        }
        
        Ok(shards)
    }
    
    /// List all disabled shards
    pub fn list_disabled_shards(&self) -> Result<Vec<String>> {
        // Create directory if it doesn't exist
        if !self.disabled_dir.exists() {
            return Ok(Vec::new());
        }
        
        let entries = fs::read_dir(&self.disabled_dir)
            .context("Failed to read disabled shards directory")?;
            
        let mut shards = Vec::new();
        
        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();
            
            // Only include .toml files
            if path.is_file() && path.extension().map_or(false, |ext| ext == "toml") {
                if let Some(name) = path.file_stem() {
                    if let Some(name_str) = name.to_str() {
                        shards.push(name_str.to_string());
                    }
                }
            }
        }
        
        Ok(shards)
    }
    
    /// Check if a shard name is valid
    fn is_valid_shard_name(&self, name: &str) -> bool {
        !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }
    
    /// Get the full path to a shard by name
    fn get_shard_path(&self, name: &str) -> PathBuf {
        self.shards_dir.join(format!("{}.toml", name))
    }
    
    /// Get the full path to a disabled shard by name
    fn get_disabled_shard_path(&self, name: &str) -> PathBuf {
        self.disabled_dir.join(format!("{}.toml", name))
    }
}

/// Create a new shard
pub fn grow_shard(name: &str, description: Option<&str>) -> Result<()> {
    let manager = ShardManager::new()?;
    manager.grow_shard(name, description)
}

/// Delete a shard
pub fn shatter_shard(name: &str, force: bool) -> Result<()> {
    let manager = ShardManager::new()?;
    manager.shatter_shard(name, force)
}

/// Disable a shard without deleting it
pub fn disable_shard(name: &str) -> Result<()> {
    let manager = ShardManager::new()?;
    manager.disable_shard(name)
}

/// Enable a previously disabled shard
pub fn enable_shard(name: &str) -> Result<()> {
    let manager = ShardManager::new()?;
    manager.enable_shard(name)
}

/// Check if a shard is protected
pub fn is_protected_shard(name: &str) -> Result<bool> {
    let manager = ShardManager::new()?;
    Ok(manager.shard_is_protected(name))
}

