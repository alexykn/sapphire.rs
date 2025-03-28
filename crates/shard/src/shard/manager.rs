#[allow(dead_code)]
use std::collections::HashMap;
use anyhow::Context;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use console::style;
use dialoguer::Confirm;
use shellexpand;
use crate::utils::{
    ShardError, ShardResult,
    log_success, log_warning, log_debug
};
use crate::core::manifest::Manifest;

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
    pub fn new() -> ShardResult<Self> {
        let shards_dir = shellexpand::tilde("~/.sapphire/shards").to_string();
        let disabled_dir = shellexpand::tilde("~/.sapphire/disabled").to_string();
        let backups_dir = shellexpand::tilde("~/.sapphire/backups").to_string();
        
        let shards_dir_path = PathBuf::from(&shards_dir);
        let disabled_dir_path = PathBuf::from(&disabled_dir);
        let backups_dir_path = PathBuf::from(&backups_dir);
        
        // Get current username for permission checks
        let current_user = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
        
        // Create directories if they don't exist
        if !shards_dir_path.exists() {
            fs::create_dir_all(&shards_dir_path)
                .with_context(|| format!("Failed to create shards directory: {}", shards_dir_path.display()))?;
        }
        
        if !disabled_dir_path.exists() {
            fs::create_dir_all(&disabled_dir_path)
                .with_context(|| format!("Failed to create disabled shards directory: {}", disabled_dir_path.display()))?;
        }
        
        Ok(Self {
            shards_dir: shards_dir_path,
            disabled_dir: disabled_dir_path,
            backups_dir: backups_dir_path,
            protected_shards: vec!["system".to_string()], // Only protect system shard by default
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
            protected_shards: vec!["system".to_string()],
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
            protected_shards: vec!["system".to_string()],
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
    
    /// Check if a shard is protected
    fn is_protected(&self, name: &str) -> ShardResult<bool> {
        // System protected shards can't be modified
        if self.protected_shards.contains(&name.to_string()) {
            return Ok(true);
        }
        
        // Check if shard exists and has protection set
        let shard_path = self.get_shard_path(name);
        if shard_path.exists() {
            if let Ok(manifest) = Manifest::from_file(shard_path.to_str().unwrap_or_default()) {
                return Ok(manifest.is_protected());
            }
        }
        
        // Check disabled path as well
        let disabled_path = self.get_disabled_shard_path(name);
        if disabled_path.exists() {
            if let Ok(manifest) = Manifest::from_file(disabled_path.to_str().unwrap_or_default()) {
                return Ok(manifest.is_protected());
            }
        }
        
        // Default to false - if shard doesn't exist yet or has no protection info
        Ok(false)
    }
    
    /// Check if current user can modify a shard
    fn can_modify_shard(&self, name: &str) -> ShardResult<bool> {
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
    fn backup_shard(&self, name: &str) -> ShardResult<PathBuf> {
        let shard_path = self.get_shard_path(name);
        if !shard_path.exists() {
            return Err(ShardError::NotFound(name.to_string()));
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
        
        log_debug(&format!("Created backup of shard '{}' at '{}'", name, backup_path.display()));
        
        Ok(backup_path)
    }
    
    /// Create a new shard
    pub fn grow_shard(&self, name: &str, description: Option<&str>) -> ShardResult<()> {
        // Validate shard name for safety
        if !self.is_valid_shard_name(name) {
            return Err(ShardError::InvalidName(name.to_string()));
        }
        
        // Check if shard already exists
        if self.shard_exists(name) {
            return Err(ShardError::AlreadyExists(name.to_string()));
        }
        
        // Create shards directory if it doesn't exist
        fs::create_dir_all(&self.shards_dir)
            .with_context(|| format!("Failed to create shards directory: {}", self.shards_dir.display()))?;
        
        let shard_path = self.get_shard_path(name);
        
        // Create default manifest
        let mut manifest = Manifest::new();
        
        // Add metadata
        manifest.metadata.name = name.to_string();
        manifest.metadata.description = description.unwrap_or("Custom shard").to_string();
        
        // Set permissions based on current user
        manifest.metadata.owner = self.current_user.clone();
        
        // Write manifest to file
        manifest.to_file(shard_path.to_str().unwrap_or_default())
            .with_context(|| format!("Failed to create shard file: {}", shard_path.display()))?;
        
        log_success(&format!("Created new shard: {}", style(name).bold()));
        
        Ok(())
    }
    
    /// Delete a shard permanently
    pub fn shatter_shard(&self, name: &str, force: bool) -> ShardResult<()> {
        // Validate shard name for safety
        if !self.is_valid_shard_name(name) {
            return Err(ShardError::InvalidName(name.to_string()));
        }
        
        // Check if the shard exists
        if !self.shard_exists(name) {
            return Err(ShardError::NotFound(name.to_string()));
        }
        
        // Check if shard is protected
        if self.is_protected(name)? {
            // If it's a system shard, hard block deletion
            if self.protected_shards.contains(&name.to_string()) {
                return Err(ShardError::Protected(name.to_string()));
            }
            
            // Otherwise, if it's just user-protected, we can override with force
            if !force {
                return Err(ShardError::Protected(name.to_string()));
            }
            
            // With force flag, we'll allow deletion but warn
            log_warning(&format!("Deleting protected shard: {} (forced)", style(name).bold()));
        }
        
        // If not forced, let the user confirm
        if !force {
            let confirm = Confirm::new()
                .with_prompt(format!("Are you sure you want to permanently delete the shard '{}'?", name))
                .default(false)
                .interact()
                .with_context(|| "Failed to get user confirmation")?;
                
            if !confirm {
                log_warning("Shard deletion cancelled");
                return Ok(());
            }
        }
        
        // First create a backup
        let backup_path = self.backup_shard(name)
            .with_context(|| format!("Failed to backup shard before deletion: {}", name))?;
        
        // Then check the shard status (active or disabled)
        let shard_path = match self.get_shard_status(name) {
            ShardStatus::Active => self.get_shard_path(name),
            ShardStatus::Disabled => self.get_disabled_shard_path(name),
            ShardStatus::NotFound => return Err(ShardError::NotFound(name.to_string())),
        };
        
        // Delete the file
        fs::remove_file(&shard_path)
            .with_context(|| format!("Failed to delete shard file: {}", shard_path.display()))?;
        
        log_success(&format!("Deleted shard: {} (backup at {})", 
            style(name).bold(), 
            style(backup_path.display()).italic()));
        
        Ok(())
    }
    
    /// Disable a shard without deleting it
    pub fn disable_shard(&self, name: &str) -> ShardResult<()> {
        // Validate shard name for safety
        if !self.is_valid_shard_name(name) {
            return Err(ShardError::InvalidName(name.to_string()));
        }
        
        // Check if the shard is protected and user doesn't have permission
        if self.is_protected(name)? && !self.can_modify_shard(name)? {
            return Err(ShardError::Protected(name.to_string()));
        }
        
        // Get source and destination paths
        let source_path = self.get_shard_path(name);
        
        // Check if shard exists
        if !source_path.exists() {
            // Check if it's already disabled
            if self.get_disabled_shard_path(name).exists() {
                log_warning(&format!("Shard '{}' is already disabled", style(name).bold()));
                return Ok(());
            }
            
            return Err(ShardError::NotFound(name.to_string()));
        }
        
        // Create backup before disabling
        let backup_path = self.backup_shard(name)
            .with_context(|| format!("Failed to create backup before disabling shard: {}", name))?;
        
        log_debug(&format!("Created backup at: {}", backup_path.display()));
        
        // Create disabled directory if it doesn't exist
        fs::create_dir_all(&self.disabled_dir)
            .with_context(|| "Failed to create disabled shards directory")?;
        
        let dest_path = self.get_disabled_shard_path(name);
        
        // Move the file to disabled directory
        fs::rename(&source_path, &dest_path)
            .with_context(|| format!("Failed to disable shard: {}", name))?;
        
        log_success(&format!("Disabled shard: {}", style(name).bold()));
        Ok(())
    }
    
    /// Enable a previously disabled shard
    pub fn enable_shard(&self, name: &str) -> ShardResult<()> {
        // Validate shard name for safety
        if !self.is_valid_shard_name(name) {
            return Err(ShardError::InvalidName(name.to_string()));
        }
        
        // Get source and destination paths
        let source_path = self.get_disabled_shard_path(name);
        
        // Check if disabled shard exists
        if !source_path.exists() {
            // Check if it's already enabled
            let enabled_path = self.get_shard_path(name);
            if enabled_path.exists() {
                log_warning(&format!("Shard '{}' is already enabled", style(name).bold()));
                return Ok(());
            }
            
            return Err(ShardError::NotFound(name.to_string()));
        }
        
        // Create shards directory if it doesn't exist
        fs::create_dir_all(&self.shards_dir)
            .with_context(|| "Failed to create shards directory")?;
        
        let dest_path = self.get_shard_path(name);
        
        // Read the manifest to update last modified information
        if let Ok(mut manifest) = Manifest::from_file(source_path.to_str().unwrap_or_default()) {
            // Update modification info
            manifest.update_modification_info();
            
            // Write the updated manifest directly to the destination
            manifest.to_file(dest_path.to_str().unwrap_or_default())
                .with_context(|| format!("Failed to write updated manifest when enabling shard: {}", name))?;
            
            // Delete the source file
            fs::remove_file(&source_path)
                .with_context(|| format!("Failed to remove disabled shard file after enabling: {}", name))?;
        } else {
            // Fall back to simple file move if manifest can't be read
            fs::rename(&source_path, &dest_path)
                .with_context(|| format!("Failed to enable shard: {}", name))?;
        }
        
        log_success(&format!("Enabled shard: {}", style(name).bold()));
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
    pub fn get_shard_info(&self, name: &str) -> ShardResult<ShardInfo> {
        let status = self.get_shard_status(name);
        let path = match status {
            ShardStatus::Active => self.get_shard_path(name),
            ShardStatus::Disabled => self.get_disabled_shard_path(name),
            ShardStatus::NotFound => return Err(ShardError::NotFound(name.to_string())),
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
    pub fn get_all_shards_info(&self) -> ShardResult<HashMap<String, ShardInfo>> {
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
    pub fn list_shards(&self) -> ShardResult<Vec<String>> {
        // Create directory if it doesn't exist
        if !self.shards_dir.exists() {
            return Ok(Vec::new());
        }
        
        let entries = fs::read_dir(&self.shards_dir)
            .with_context(|| "Failed to read shards directory")?;
            
        let mut shards = Vec::new();
        
        for entry in entries {
            let entry = entry.with_context(|| "Failed to read directory entry")?;
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
    pub fn list_disabled_shards(&self) -> ShardResult<Vec<String>> {
        // Create directory if it doesn't exist
        if !self.disabled_dir.exists() {
            return Ok(Vec::new());
        }
        
        let entries = fs::read_dir(&self.disabled_dir)
            .with_context(|| "Failed to read disabled shards directory")?;
            
        let mut shards = Vec::new();
        
        for entry in entries {
            let entry = entry.with_context(|| "Failed to read directory entry")?;
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
pub fn grow_shard(name: &str, description: Option<&str>) -> ShardResult<()> {
    let manager = ShardManager::new()?;
    manager.grow_shard(name, description)
}

/// Delete a shard
pub fn shatter_shard(name: &str, force: bool) -> ShardResult<()> {
    let manager = ShardManager::new()?;
    manager.shatter_shard(name, force)
}

/// Disable a shard without deleting it
pub fn disable_shard(name: &str) -> ShardResult<()> {
    let manager = ShardManager::new()?;
    manager.disable_shard(name)
}

/// Enable a previously disabled shard
pub fn enable_shard(name: &str) -> ShardResult<()> {
    let manager = ShardManager::new()?;
    manager.enable_shard(name)
}

/// Check if a shard is protected
pub fn is_protected_shard(name: &str) -> ShardResult<bool> {
    let manager = ShardManager::new()?;
    manager.is_protected(name)
}

