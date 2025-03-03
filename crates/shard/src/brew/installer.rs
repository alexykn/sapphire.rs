//! Homebrew package installation and management functionality.
//!
//! This module handles operations that interact with or modify the local package state
//! such as installing, uninstalling, updating, and upgrading packages. It ensures that
//! all user inputs are properly validated before execution to prevent command injection.

use anyhow::Result;
use crate::brew::core::BrewCore;
use crate::brew::validate as validation;

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
    pub fn add_tap(&self, tap: &str) -> Result<()> {
        // Validate tap name before execution
        let validated_tap = validation::validate_tap_name(tap)?;
        
        self.core.execute_brew_command(&["tap", validated_tap])?;
        Ok(())
    }
    
    /// Install a Homebrew formula
    pub fn install_formula(&self, formula: &str, options: &[String]) -> Result<()> {
        // Validate formula name before execution
        let validated_formula = validation::validate_package_name(formula)?;
        validation::validate_options(options)?;
        
        // Create a vector of &str for the options
        let option_strs: Vec<&str> = options.iter().map(AsRef::as_ref).collect();
        
        self.core.execute_brew_command_with_args(&["install", validated_formula], &option_strs)?;
        Ok(())
    }
    
    /// Install a Homebrew cask
    pub fn install_cask(&self, cask: &str, options: &[String]) -> Result<()> {
        // Validate cask name before execution
        let validated_cask = validation::validate_package_name(cask)?;
        validation::validate_options(options)?;
        
        // Create a vector of &str for the options
        let option_strs: Vec<&str> = options.iter().map(AsRef::as_ref).collect();
        
        self.core.execute_brew_command_with_args(&["install", "--cask", validated_cask], &option_strs)?;
        Ok(())
    }

    /// Get a list of all currently installed formulae
    pub fn get_installed_formulae(&self) -> Result<Vec<String>> {
        let output = self.core.execute_brew_command(&["list", "--formula"])?;
        Ok(self.core.parse_list_output(output))
    }

    /// Get a list of all currently installed casks
    pub fn get_installed_casks(&self) -> Result<Vec<String>> {
        let output = self.core.execute_brew_command(&["list", "--cask"])?;
        Ok(self.core.parse_list_output(output))
    }

    /// Get a list of all currently installed taps
    pub fn get_installed_taps(&self) -> Result<Vec<String>> {
        let output = self.core.execute_brew_command(&["tap"])?;
        Ok(self.core.parse_list_output(output))
    }

    /// Perform a batch install of multiple formulae at once
    ///
    /// # Security
    ///
    /// All package names are validated individually before execution
    pub fn batch_install_formulae(&self, formulae: &[String]) -> Result<()> {
        if formulae.is_empty() {
            return Ok(());
        }
        
        // Validate all package names
        let mut validated_formulae = Vec::with_capacity(formulae.len());
        for formula in formulae {
            validated_formulae.push(validation::validate_package_name(formula)?);
        }
        
        // Create a vec with "install" followed by all formula names
        let mut args = vec!["install"];
        args.extend(validated_formulae);
        
        self.core.execute_brew_command(&args)?;
        Ok(())
    }

    /// Perform a batch install of multiple casks at once
    ///
    /// # Security
    ///
    /// All cask names are validated individually before execution
    pub fn batch_install_casks(&self, casks: &[String]) -> Result<()> {
        if casks.is_empty() {
            return Ok(());
        }
        
        // Validate all package names
        let mut validated_casks = Vec::with_capacity(casks.len());
        for cask in casks {
            validated_casks.push(validation::validate_package_name(cask)?);
        }
        
        // Create a vec with install --cask followed by all cask names
        let mut args = vec!["install", "--cask"];
        args.extend(validated_casks);
        
        self.core.execute_brew_command(&args)?;
        Ok(())
    }

    /// Perform a batch upgrade of multiple formulae at once
    ///
    /// # Security
    ///
    /// All package names are validated individually before execution
    pub fn batch_upgrade_formulae(&self, formulae: &[String]) -> Result<()> {
        if formulae.is_empty() {
            return Ok(());
        }
        
        // Validate all package names
        let mut validated_formulae = Vec::with_capacity(formulae.len());
        for formula in formulae {
            validated_formulae.push(validation::validate_package_name(formula)?);
        }
        
        let mut args = vec!["upgrade"];
        args.extend(validated_formulae);
        
        self.core.execute_brew_command(&args)?;
        Ok(())
    }

    /// Perform a batch upgrade of multiple casks at once
    ///
    /// # Security
    ///
    /// All cask names are validated individually before execution
    pub fn batch_upgrade_casks(&self, casks: &[String]) -> Result<()> {
        if casks.is_empty() {
            return Ok(());
        }
        
        // Validate all package names
        let mut validated_casks = Vec::with_capacity(casks.len());
        for cask in casks {
            validated_casks.push(validation::validate_package_name(cask)?);
        }
        
        let mut args = vec!["upgrade", "--cask"];
        args.extend(validated_casks);
        
        self.core.execute_brew_command(&args)?;
        Ok(())
    }

    /// Upgrade a formula with custom options
    pub fn upgrade_formula_with_options(&self, formula: &str, options: &[String]) -> Result<()> {
        // Validate formula name and options
        let validated_formula = validation::validate_package_name(formula)?;
        validation::validate_options(options)?;
        
        // Create a vector of &str for the options
        let option_strs: Vec<&str> = options.iter().map(AsRef::as_ref).collect();
        
        self.core.execute_brew_command_with_args(&["upgrade", validated_formula], &option_strs)?;
        Ok(())
    }

    /// Upgrade a cask with custom options
    pub fn upgrade_cask_with_options(&self, cask: &str, options: &[String]) -> Result<()> {
        // Validate cask name and options
        let validated_cask = validation::validate_package_name(cask)?;
        validation::validate_options(options)?;
        
        // Create a vector of &str for the options
        let option_strs: Vec<&str> = options.iter().map(AsRef::as_ref).collect();
        
        self.core.execute_brew_command_with_args(&["upgrade", "--cask", validated_cask], &option_strs)?;
        Ok(())
    }

    /// Uninstall a formula
    pub fn uninstall_formula(&self, formula: &str, force: bool) -> Result<()> {
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
    pub fn uninstall_cask(&self, cask: &str, force: bool) -> Result<()> {
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
    pub fn get_dependency_packages(&self) -> Result<Vec<String>> {
        let output = self.core.execute_brew_command(&["list", "--installed-as-dependency"])?;
        Ok(self.core.parse_list_output(output))
    }

    /// Run cleanup
    pub fn cleanup(&self, prune_all: bool) -> Result<()> {
        let mut args = vec!["cleanup"];
        
        if prune_all {
            args.push("--prune=all");
        }
        
        self.core.execute_brew_command(&args)?;
        Ok(())
    }
}

/// Get a default installer instance
pub fn get_installer() -> BrewInstaller {
    BrewInstaller::new()
} 