//! Homebrew package installation and management functionality.
//!
//! This module handles operations that interact with or modify the local package state
//! such as installing, uninstalling, updating, and upgrading packages. It ensures that
//! all user inputs are properly validated before execution to prevent command injection.

use crate::ShardResult;
use crate::brew::core::BrewCore;
use crate::brew::validate as validation;
use crate::utils::{log_warning, log_error};

/// Handles installation, uninstallation, updates, and other operations
/// that modify the local package state
pub struct BrewInstaller {
    core: BrewCore,
}

impl BrewInstaller {
    /// Create a new installer with default brew core
    pub fn new() -> Self {
        Self {
            core: BrewCore::new(),
        }
    }
    
    /// Create a new installer with a custom brew core
    pub fn with_core(core: BrewCore) -> Self {
        Self { core }
    }
    
    /// Add a Homebrew tap
    pub fn add_tap(&self, tap: &str) -> ShardResult<()> {
        // Validate tap name before execution
        let validated_tap = validation::validate_tap_name(tap)?;
        
        self.core.execute_brew_command(&["tap", validated_tap])?;
        Ok(())
    }
    
    /// Install a Homebrew formula
    pub fn install_formula(&self, formula: &str, options: &[String]) -> ShardResult<()> {
        // Validate formula name before execution
        let validated_formula = validation::validate_package_name(formula)?;
        validation::validate_options(options)?;
        
        // Create a vector of &str for the options
        let option_strs: Vec<&str> = options.iter().map(AsRef::as_ref).collect();
        
        self.core.execute_brew_command_with_args(&["install", validated_formula], &option_strs)?;
        Ok(())
    }
    
    /// Install a Homebrew cask
    pub fn install_cask(&self, cask: &str, options: &[String]) -> ShardResult<()> {
        // Validate cask name before execution
        let validated_cask = validation::validate_package_name(cask)?;
        validation::validate_options(options)?;
        
        // Create a vector of &str for the options
        let option_strs: Vec<&str> = options.iter().map(AsRef::as_ref).collect();
        
        self.core.execute_brew_command_with_args(&["install", "--cask", validated_cask], &option_strs)?;
        Ok(())
    }

    /// Get a list of all currently installed formulae
    pub fn get_installed_formulae(&self) -> ShardResult<Vec<String>> {
        let output = self.core.execute_brew_command(&["list", "--formula"])?;
        Ok(self.core.parse_list_output(output))
    }

    /// Get a list of all currently installed casks
    pub fn get_installed_casks(&self) -> ShardResult<Vec<String>> {
        let output = self.core.execute_brew_command(&["list", "--cask"])?;
        Ok(self.core.parse_list_output(output))
    }

    /// Get a list of all currently installed taps
    pub fn get_installed_taps(&self) -> ShardResult<Vec<String>> {
        let output = self.core.execute_brew_command(&["tap"])?;
        Ok(self.core.parse_list_output(output))
    }

    /// Perform a batch install of multiple formulae at once
    ///
    /// # Security
    ///
    /// All package names are validated individually before execution
    pub fn batch_install_formulae(&self, formulae: &[String]) -> ShardResult<()> {
        if formulae.is_empty() {
            return Ok(());
        }
        
        // Install formulae one by one for better error handling
        for formula in formulae {
            let validated_formula = validation::validate_package_name(formula)?;
            
            // Try to install each formula individually
            let result = self.core.execute_brew_command(&["install", validated_formula]);
            
            if let Err(e) = result {
                // Log the error but continue with other formulae
                let error_str = e.to_string();
                if error_str.contains("already installed") {
                    log_warning(&format!("Skipping {}: {}", formula, error_str));
                    continue;
                } else {
                    log_error(&format!("Error installing {}: {}", formula, error_str));
                    // Don't fail the entire process for one formula
                    continue;
                }
            }
        }
        
        Ok(())
    }

    /// Perform a batch install of multiple casks at once
    ///
    /// # Security
    ///
    /// All cask names are validated individually before execution
    pub fn batch_install_casks(&self, casks: &[String]) -> ShardResult<()> {
        if casks.is_empty() {
            return Ok(());
        }
        
        // Install casks one by one for better error handling
        for cask in casks {
            let validated_cask = validation::validate_package_name(cask)?;
            
            // Try to install each cask individually
            let result = self.core.execute_brew_command(&["install", "--cask", validated_cask]);
            
            if let Err(e) = result {
                // Log the error but continue with other casks
                if e.to_string().contains("already a Binary at") || 
                   e.to_string().contains("already installed") {
                    // If it's already installed or there's a binary conflict, just skip it
                    log_warning(&format!("Skipping {}: {}", cask, e));
                    continue;
                } else {
                    // For other errors, log but continue
                    log_error(&format!("Error installing {}: {}", cask, e));
                    // Don't fail the entire process for one cask
                    continue;
                }
            }
        }
        
        Ok(())
    }

    /// Perform a batch upgrade of multiple formulae at once
    ///
    /// # Security
    ///
    /// All package names are validated individually before execution
    pub fn batch_upgrade_formulae(&self, formulae: &[String]) -> ShardResult<()> {
        if formulae.is_empty() {
            return Ok(());
        }
        
        // Upgrade formulae one by one for better error handling
        for formula in formulae {
            let validated_formula = validation::validate_package_name(formula)?;
            
            // Attempt to upgrade each formula individually
            let result = self.core.execute_brew_command(&["upgrade", validated_formula]);
            
            if let Err(e) = result {
                // Log but continue with other formulae
                log_warning(&format!("Error upgrading {}: {}", formula, e));
                continue;
            }
        }
        
        Ok(())
    }

    /// Perform a batch upgrade of multiple casks at once
    ///
    /// # Security
    ///
    /// All cask names are validated individually before execution
    pub fn batch_upgrade_casks(&self, casks: &[String]) -> ShardResult<()> {
        if casks.is_empty() {
            return Ok(());
        }
        
        // Upgrade casks one by one for better error handling
        for cask in casks {
            let validated_cask = validation::validate_package_name(cask)?;
            
            // Attempt to upgrade each cask individually
            let result = self.core.execute_brew_command(&["upgrade", "--cask", validated_cask]);
            
            if let Err(e) = result {
                // Log but continue with other casks
                log_warning(&format!("Error upgrading {}: {}", cask, e));
                continue;
            }
        }
        
        Ok(())
    }

    /// Upgrade a formula with custom options
    pub fn upgrade_formula_with_options(&self, formula: &str, options: &[String]) -> ShardResult<()> {
        // Validate formula name and options
        let validated_formula = validation::validate_package_name(formula)?;
        validation::validate_options(options)?;
        
        // Create a vector of &str for the options
        let option_strs: Vec<&str> = options.iter().map(AsRef::as_ref).collect();
        
        self.core.execute_brew_command_with_args(&["upgrade", validated_formula], &option_strs)?;
        Ok(())
    }

    /// Upgrade a cask with custom options
    pub fn upgrade_cask_with_options(&self, cask: &str, options: &[String]) -> ShardResult<()> {
        // Validate cask name and options
        let validated_cask = validation::validate_package_name(cask)?;
        validation::validate_options(options)?;
        
        // Create a vector of &str for the options
        let option_strs: Vec<&str> = options.iter().map(AsRef::as_ref).collect();
        
        self.core.execute_brew_command_with_args(&["upgrade", "--cask", validated_cask], &option_strs)?;
        Ok(())
    }

    /// Uninstall a formula
    pub fn uninstall_formula(&self, formula: &str, force: bool) -> ShardResult<()> {
        // Validate formula name
        let validated_formula = validation::validate_package_name(formula)?;
        
        let mut args = vec!["uninstall", "--formula", validated_formula];
        
        if force {
            args.push("--force");
        }
        
        self.core.execute_brew_command(&args)?;
        Ok(())
    }

    /// Uninstall a cask
    pub fn uninstall_cask(&self, cask: &str, force: bool) -> ShardResult<()> {
        // Validate cask name
        let validated_cask = validation::validate_package_name(cask)?;
        
        let mut args = vec!["uninstall", "--cask", validated_cask];
        
        if force {
            args.push("--force");
        }
        
        self.core.execute_brew_command(&args)?;
        Ok(())
    }

    /// Get a list of all packages installed as dependencies
    pub fn get_dependency_packages(&self) -> ShardResult<Vec<String>> {
        let output = self.core.execute_brew_command(&["list", "--installed-as-dependency"])?;
        Ok(self.core.parse_list_output(output))
    }

    /// Run cleanup
    pub fn cleanup(&self, prune_all: bool) -> ShardResult<()> {
        let mut args = vec!["cleanup"];
        
        if prune_all {
            args.push("--prune=all");
        }
        
        self.core.execute_brew_command(&args)?;
        Ok(())
    }

    pub fn batch_install_formulas(&self, formulas: &[String], args: &[&str]) -> Result<(), String> {
        if formulas.is_empty() {
            return Ok(());
        }

        // Validate all package names before installation to prevent command injection
        for formula in formulas {
            match validation::validate_package_name(formula) {
                Ok(_) => {},
                Err(e) => return Err(e.to_string()),
            }
        }

        // Install each formula individually to handle errors gracefully
        for formula in formulas {
            let validated_formula = formula.as_str();
            
            // Create command: brew install <args> <formula>
            let mut cmd = vec!["install"];
            cmd.extend_from_slice(args);
            cmd.push(validated_formula);
            
            if let Err(e) = self.core.execute_brew_command(&cmd) {
                let error_str = e.to_string();
                if error_str.contains("already installed") {
                    log_warning(&format!("Skipping {}: {}", formula, error_str));
                    continue;
                } else {
                    log_error(&format!("Error installing {}: {}", formula, error_str));
                }
            }
        }

        Ok(())
    }
}

/// Get a default installer instance
pub fn get_installer() -> BrewInstaller {
    BrewInstaller::new()
} 