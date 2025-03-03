//! Primary client interface for Homebrew CLI operations.
//!
//! This module provides a unified API for all Homebrew operations, delegating to specialized
//! modules for implementation. It acts as a facade that coordinates between the core execution,
//! package installation, and search functionality.
//! 
//! The client maintains the same API interface regardless of internal implementation changes,
//! ensuring backward compatibility while supporting proper separation of concerns.
//! All operations enforce proper input validation to prevent command injection.

use crate::utils::ShardResult;
use crate::brew::core::BrewCore;
use crate::brew::installer::BrewInstaller;
use crate::brew::search::BrewSearcher;

/// Homebrew client for interacting with brew CLI
pub struct BrewClient {
    /// Core execution engine
    core: BrewCore,
    /// Package installer
    installer: BrewInstaller,
    /// Package searcher
    searcher: BrewSearcher,
}

impl BrewClient {
    /// Create a new client with the default brew path
    pub fn new() -> Self {
        let core = BrewCore::new();
        Self {
            installer: BrewInstaller::with_core(core.clone()),
            searcher: BrewSearcher::with_core(core.clone()),
            core,
        }
    }
    
    /// Create a new client with a custom brew path
    pub fn with_path(brew_path: String) -> Self {
        let core = BrewCore::with_path(brew_path);
        Self {
            installer: BrewInstaller::with_core(core.clone()),
            searcher: BrewSearcher::with_core(core.clone()),
            core,
        }
    }
    
    /// Enable debug logging
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.core = self.core.with_debug(debug);
        self.installer = BrewInstaller::with_core(self.core.clone());
        self.searcher = BrewSearcher::with_core(self.core.clone());
        self
    }
    
    /// Set a timeout for commands in seconds
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.core = self.core.with_timeout(seconds);
        self.installer = BrewInstaller::with_core(self.core.clone());
        self.searcher = BrewSearcher::with_core(self.core.clone());
        self
    }

    // Installer delegated methods
    
    /// Add a Homebrew tap
    pub fn add_tap(&self, tap: &str) -> ShardResult<()> {
        self.installer.add_tap(tap)
    }
    
    /// Install a Homebrew formula
    pub fn install_formula(&self, formula: &str, options: &[String]) -> ShardResult<()> {
        self.installer.install_formula(formula, options)
    }
    
    /// Install a Homebrew cask
    pub fn install_cask(&self, cask: &str, options: &[String]) -> ShardResult<()> {
        self.installer.install_cask(cask, options)
    }

    /// Get a list of all currently installed formulae
    pub fn get_installed_formulae(&self) -> ShardResult<Vec<String>> {
        self.installer.get_installed_formulae()
    }

    /// Get a list of all currently installed casks
    pub fn get_installed_casks(&self) -> ShardResult<Vec<String>> {
        self.installer.get_installed_casks()
    }

    /// Get a list of all currently installed taps
    pub fn get_installed_taps(&self) -> ShardResult<Vec<String>> {
        self.installer.get_installed_taps()
    }

    /// Perform a batch install of multiple formulae at once
    pub fn batch_install_formulae(&self, formulae: &[String]) -> ShardResult<()> {
        self.installer.batch_install_formulae(formulae)
    }

    /// Perform a batch install of multiple casks at once
    pub fn batch_install_casks(&self, casks: &[String]) -> ShardResult<()> {
        self.installer.batch_install_casks(casks)
    }

    /// Perform a batch upgrade of multiple formulae at once
    pub fn batch_upgrade_formulae(&self, formulae: &[String]) -> ShardResult<()> {
        self.installer.batch_upgrade_formulae(formulae)
    }

    /// Perform a batch upgrade of multiple casks at once
    pub fn batch_upgrade_casks(&self, casks: &[String]) -> ShardResult<()> {
        self.installer.batch_upgrade_casks(casks)
    }

    /// Upgrade a formula with custom options
    pub fn upgrade_formula_with_options(&self, formula: &str, options: &[String]) -> ShardResult<()> {
        self.installer.upgrade_formula_with_options(formula, options)
    }

    /// Upgrade a cask with custom options
    pub fn upgrade_cask_with_options(&self, cask: &str, options: &[String]) -> ShardResult<()> {
        self.installer.upgrade_cask_with_options(cask, options)
    }

    /// Uninstall a formula
    pub fn uninstall_formula(&self, formula: &str, force: bool) -> ShardResult<()> {
        self.installer.uninstall_formula(formula, force)
    }

    /// Uninstall a cask
    pub fn uninstall_cask(&self, cask: &str, force: bool) -> ShardResult<()> {
        self.installer.uninstall_cask(cask, force)
    }

    /// Get a list of all packages installed as dependencies
    pub fn get_dependency_packages(&self) -> ShardResult<Vec<String>> {
        self.installer.get_dependency_packages()
    }

    /// Run cleanup
    pub fn cleanup(&self, prune_all: bool) -> ShardResult<()> {
        self.installer.cleanup(prune_all)
    }
    
    // Searcher delegated methods
    
    /// Search for packages
    pub fn search(&self, query: &str, formula_only: bool, cask_only: bool) -> ShardResult<Vec<String>> {
        self.searcher.search(query, formula_only, cask_only)
    }
    
    /// Get detailed information about a formula
    pub fn get_formula_info(&self, formula: &str) -> ShardResult<crate::brew::search::FormulaInfo> {
        self.searcher.get_formula_info(formula)
    }
    
    /// Get detailed information about a cask
    pub fn get_cask_info(&self, cask: &str) -> ShardResult<crate::brew::search::CaskInfo> {
        self.searcher.get_cask_info(cask)
    }
}