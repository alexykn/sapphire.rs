use crate::ShardResult;
use crate::core::manifest::{PackageState, Formula, Cask};
use crate::brew::{BrewClient, get_client};
use crate::utils::{log_step, log_success, log_error, log_warning};

/// Represents the type of package being managed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageType {
    Formula,
    Cask,
}

impl PackageType {
    /// Get the string representation of the package type
    pub fn as_str(&self) -> &'static str {
        match self {
            PackageType::Formula => "formula",
            PackageType::Cask => "cask",
        }
    }
}

/// Represents a package operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageOperation {
    Install,
    Upgrade,
    Uninstall,
}

impl PackageOperation {
    /// Get the string representation of the operation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Install => "Installing",
            Self::Upgrade => "Upgrading",
            Self::Uninstall => "Uninstalling",
        }
    }
}

/// Structure to hold the results of package processing
pub struct PackageProcessResult {
    pub to_install: Vec<String>,
    pub to_upgrade: Vec<String>,
    pub with_options: Vec<(String, Vec<String>)>,
    pub to_uninstall: Vec<String>,
}

/// Trait for package information
pub trait PackageInfo {
    fn state(&self) -> PackageState;
    fn options(&self) -> &[String];
    fn name(&self) -> &str;
}

impl PackageInfo for Formula {
    fn state(&self) -> PackageState {
        self.state.clone()
    }
    
    fn options(&self) -> &[String] {
        &self.options
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

impl PackageInfo for &str {
    fn state(&self) -> PackageState {
        PackageState::Latest
    }
    
    fn options(&self) -> &[String] {
        &[]
    }
    
    fn name(&self) -> &str {
        self
    }
}

impl PackageInfo for String {
    fn state(&self) -> PackageState {
        PackageState::Latest
    }
    
    fn options(&self) -> &[String] {
        &[]
    }
    
    fn name(&self) -> &str {
        self
    }
}

impl PackageInfo for Cask {
    fn state(&self) -> PackageState {
        self.state.clone()
    }
    
    fn options(&self) -> &[String] {
        &self.options
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// Generic package processor to handle both formulae and casks with similar logic
pub struct PackageProcessor {
    pub package_type: PackageType,
    pub installed_packages: Vec<String>,
    pub suppress_messages: bool,
    brew_client: BrewClient,
}

impl PackageProcessor {
    /// Create a new processor with the given package type and installed packages
    pub fn new(package_type: PackageType, installed_packages: Vec<String>, suppress_messages: bool) -> Self {
        Self {
            package_type,
            installed_packages,
            suppress_messages,
            brew_client: get_client(),
        }
    }
    
    /// Check if a package is installed
    pub fn is_installed(&self, name: &str) -> bool {
        self.installed_packages.iter().any(|p| p == name)
    }
    
    /// Process packages and determine what actions need to be taken
    pub fn process_packages<T>(&self, packages: &[T]) -> ShardResult<PackageProcessResult>
    where 
        T: PackageInfo,
    {
        let mut result = PackageProcessResult {
            to_install: Vec::new(),
            to_upgrade: Vec::new(),
            with_options: Vec::new(),
            to_uninstall: Vec::new(),
        };
        
        for package in packages {
            let name = package.name();
            let state = package.state();
            let options = package.options();
            
            match state {
                PackageState::Latest => {
                    let is_installed = self.is_installed(name);
                    
                    if !options.is_empty() {
                        // Handle packages with custom options
                        result.with_options.push((name.to_string(), options.to_vec()));
                    } else if is_installed {
                        // Package is installed, add to upgrade list
                        result.to_upgrade.push(name.to_string());
                    } else {
                        // Package is not installed, add to install list
                        result.to_install.push(name.to_string());
                    }
                },
                PackageState::Present => {
                    // Just ensure the package is installed, don't upgrade
                    if !self.is_installed(name) {
                        if !options.is_empty() {
                            result.with_options.push((name.to_string(), options.to_vec()));
                        } else {
                            result.to_install.push(name.to_string());
                        }
                    }
                },
                PackageState::Absent => {
                    // Package should be uninstalled
                    if self.is_installed(name) {
                        result.to_uninstall.push(name.to_string());
                    }
                },
            }
        }
        
        Ok(result)
    }
    
    /// Execute operations on the packages based on the processed results
    pub fn execute_operations(&self, result: &PackageProcessResult, dry_run: bool) -> ShardResult<()> {
        let pkg_type_str = self.package_type.as_str();

        // --- Dry Run Handling ---
        if dry_run {
            if !result.to_install.is_empty() {
                 log_step(&format!("Would install {} {}(s): {}", result.to_install.len(), pkg_type_str, result.to_install.join(", ")));
            }
            // Skip upgrade information in dry-run mode as it's not relevant for manifests
            // Only show packages that would be newly installed or uninstalled
            for (name, options) in &result.with_options {
                 // Only show installation messages, not upgrades
                 if !self.is_installed(name) {
                     log_step(&format!("Would install {} {} with options: {}", pkg_type_str, name, options.join(" ")));
                 }
            }
            if !result.to_uninstall.is_empty() {
                 log_step(&format!("Would uninstall {} {}(s): {}", result.to_uninstall.len(), pkg_type_str, result.to_uninstall.join(", ")));
            }
            return Ok(());
        }

        // --- Actual Execution ---

        // Process installations (batch)
        if !result.to_install.is_empty() {
            match self.package_type {
                PackageType::Formula => {
                    // Use our improved method with better error handling
                    if let Err(e) = self.brew_client.batch_install_formulae(&result.to_install) {
                        log_warning(&format!("Some formula installations may have failed: {}", e));
                    }
                },
                PackageType::Cask => {
                    // This already has improved error handling
                    if let Err(e) = self.brew_client.batch_install_casks(&result.to_install) {
                        log_warning(&format!("Some cask installations may have failed: {}", e));
                    }
                },
            }
        }

        // Process upgrades (batch) - with improved error handling
        if !result.to_upgrade.is_empty() {
            match self.package_type {
                PackageType::Formula => {
                    if let Err(e) = self.brew_client.batch_upgrade_formulae(&result.to_upgrade) {
                        log_warning(&format!("Some formula upgrades may have failed: {}", e));
                    }
                },
                PackageType::Cask => {
                    if let Err(e) = self.brew_client.batch_upgrade_casks(&result.to_upgrade) {
                        log_warning(&format!("Some cask upgrades may have failed: {}", e));
                    }
                },
            }
        }

        // Process packages with options (individual)
        for (name, options) in &result.with_options {
            let is_installed = self.is_installed(name);
            match self.package_type {
                PackageType::Formula => {
                    if is_installed {
                        if let Err(e) = self.brew_client.upgrade_formula_with_options(name, options) {
                            log_warning(&format!("Failed to upgrade formula {} with options: {}", name, e));
                        }
                    } else {
                        if let Err(e) = self.brew_client.install_formula(name, options) {
                            log_warning(&format!("Failed to install formula {} with options: {}", name, e));
                        }
                    }
                }
                PackageType::Cask => {
                    if is_installed {
                        if let Err(e) = self.brew_client.upgrade_cask_with_options(name, options) {
                            log_warning(&format!("Failed to upgrade cask {} with options: {}", name, e));
                        } 
                    } else {
                        if let Err(e) = self.brew_client.install_cask(name, options) {
                            log_warning(&format!("Failed to install cask {} with options: {}", name, e));
                        }
                    }
                }
            }
        }

        // Process uninstallations (individual)
        if !result.to_uninstall.is_empty() {
             log_step(&format!("Processing {} {} uninstalls...", result.to_uninstall.len(), pkg_type_str));
            for name in &result.to_uninstall {
                match self.package_type {
                     PackageType::Formula => {
                         if let Err(e) = self.brew_client.uninstall_formula(name, true) {
                              log_warning(&format!("Failed to uninstall formula {}: {}", name, e));
                         }
                     },
                     PackageType::Cask => {
                         if let Err(e) = self.brew_client.uninstall_cask(name, true) {
                              log_warning(&format!("Failed to uninstall cask {}: {}", name, e));
                         }
                     },
                }
            }
        }

        Ok(())
    }
    
    /// Create a new processor for formulae
    pub fn for_formulae(suppress_messages: bool) -> ShardResult<Self> {
        let brew_client = get_client();
        let installed_packages = brew_client.get_installed_formulae()?;
        Ok(Self {
            package_type: PackageType::Formula,
            installed_packages,
            suppress_messages,
            brew_client,
        })
    }
    
    /// Create a new processor for casks
    pub fn for_casks(suppress_messages: bool) -> ShardResult<Self> {
        let brew_client = get_client();
        let installed_packages = brew_client.get_installed_casks()?;
        Ok(Self {
            package_type: PackageType::Cask,
            installed_packages,
            suppress_messages,
            brew_client,
        })
    }
    
    /// Uninstall a package using this processor's package type
    pub fn uninstall(&self, name: &str, force: bool) -> ShardResult<()> {
        match self.package_type {
            PackageType::Formula => self.brew_client.uninstall_formula(name, force),
            PackageType::Cask => self.brew_client.uninstall_cask(name, force),
        }
    }
}

// The following are convenience functions that directly use the BrewClient
// These could be imported directly from brew::get_client() in consuming code
// but are kept for backward compatibility

/// Get a list of all currently installed formulae
pub fn get_installed_formulae() -> ShardResult<Vec<String>> {
    get_client().get_installed_formulae()
}

/// Get a list of all currently installed casks
pub fn get_installed_casks() -> ShardResult<Vec<String>> {
    get_client().get_installed_casks()
}

/// Get a list of all currently installed taps
pub fn get_installed_taps() -> ShardResult<Vec<String>> {
    get_client().get_installed_taps()
}

/// Get a list of all packages installed as dependencies
pub fn get_dependency_packages() -> ShardResult<Vec<String>> {
    get_client().get_dependency_packages()
}

/// Get a list of explicitly installed packages (both formulae and casks, excluding dependencies)
pub fn get_all_main_packages() -> ShardResult<(Vec<String>, Vec<String>)> {
    let brew_client = get_client();
    let main_formulae = brew_client.get_installed_formulae()?;
    let main_casks = brew_client.get_installed_casks()?;
    let dependency_packages = brew_client.get_dependency_packages()?;
    
    // Filter out dependencies
    let main_formulae = main_formulae
        .into_iter()
        .filter(|f| !dependency_packages.contains(f))
        .collect();
    
    Ok((main_formulae, main_casks))
}

/// Add a tap to Homebrew
pub fn add_tap(name: &str) -> ShardResult<()> {
    get_client().add_tap(name)
}

/// Run Homebrew cleanup
pub fn run_cleanup() -> ShardResult<()> {
    log_step("Running final cleanup...");
    match get_client().cleanup(true) {
        Ok(_) => {
            log_success("Final cleanup completed");
            Ok(())
        },
        Err(e) => {
            log_error(&format!("Error cleaning up Homebrew packages: {}", e));
            Err(e)
        }
    }
}
