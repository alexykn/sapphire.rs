use crate::ShardResult;
use crate::utils::{ShardError, ResultExt};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use crate::core::manifest::{PackageState, Formula, Cask};
use crate::brew::client::BrewClient;
use std::process::Command;

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

/// Get a new BrewClient instance
pub fn get_brew_client() -> BrewClient {
    BrewClient::new()
}

/// Helper function to execute a command with proper error handling and messaging
pub fn execute_brew_command<F, S>(
    operation: PackageOperation, 
    package_type: &str, 
    package_name: &str, 
    has_options: bool,
    command: F,
    suppress_already_installed_messages: bool
) -> ShardResult<()> 
where
    F: FnOnce() -> ShardResult<()>,
    S: AsRef<str>,
{
    let options_text = if has_options { " with custom options" } else { "" };
    
    // Create progress bar for individual package operations
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    
    pb.set_message(format!("{} {}{}: {}", 
        operation.as_str(), 
        package_type, 
        options_text,
        style(package_name).bold()));
    
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    
    match command() {
        Ok(_) => {
            pb.finish_with_message(format!("{} {} {}{}: {}", 
                style("✓").green().bold(),
                operation.as_str(), 
                package_type, 
                options_text,
                style(package_name).bold()));
            Ok(())
        },
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("already installed") {
                if !suppress_already_installed_messages {
                    let status_msg = if operation == PackageOperation::Upgrade {
                        "is already up-to-date"
                    } else {
                        "is already installed"
                    };
                    pb.finish_with_message(format!("{} {} {}",
                        style(package_type).bold(),
                        style(package_name).bold(),
                        status_msg));
                } else {
                    pb.finish();
                }
                Ok(())
            } else {
                pb.finish_with_message(format!("{} Error {} {}: {}", 
                    style("✗").red().bold(),
                    operation.as_str().to_lowercase(), 
                    style(package_name).bold(), 
                    e));
                Err(e)
            }
        }
    }
}

/// Generic function for package operations with options
pub fn package_operation_with_options(
    package_type: PackageType,
    operation: PackageOperation, 
    name: &str, 
    options: &[String], 
    suppress_already_installed_messages: bool
) -> ShardResult<()> {
    let type_str = package_type.as_str();
    let has_options = !options.is_empty();
    
    execute_brew_command::<_, String>(
        operation, 
        type_str, 
        name, 
        has_options,
        || {
            match (package_type, operation) {
                (PackageType::Formula, PackageOperation::Install) => get_brew_client().install_formula(name, options),
                (PackageType::Formula, PackageOperation::Upgrade) => get_brew_client().upgrade_formula_with_options(name, options),
                (PackageType::Cask, PackageOperation::Install) => get_brew_client().install_cask(name, options),
                (PackageType::Cask, PackageOperation::Upgrade) => get_brew_client().upgrade_cask_with_options(name, options),
                _ => Err(ShardError::BrewError(format!("Unsupported operation '{:?}' for {}", operation, type_str))),
            }
        },
        suppress_already_installed_messages
    )
}

/// Helper function to install a formula with options
pub fn install_formula_with_options(
    name: &str, 
    options: &[String], 
    suppress_already_installed_messages: bool
) -> ShardResult<()> {
    package_operation_with_options(
        PackageType::Formula,
        PackageOperation::Install,
        name,
        options,
        suppress_already_installed_messages
    )
}

/// Helper function to upgrade a formula with options
pub fn upgrade_formula_with_options(
    name: &str, 
    options: &[String], 
    suppress_already_installed_messages: bool
) -> ShardResult<()> {
    package_operation_with_options(
        PackageType::Formula,
        PackageOperation::Upgrade,
        name,
        options,
        suppress_already_installed_messages
    )
}

/// Helper function to install a cask with options
pub fn install_cask_with_options(
    name: &str, 
    options: &[String], 
    suppress_already_installed_messages: bool
) -> ShardResult<()> {
    package_operation_with_options(
        PackageType::Cask,
        PackageOperation::Install,
        name,
        options,
        suppress_already_installed_messages
    )
}

/// Helper function to upgrade a cask with options
pub fn upgrade_cask_with_options(
    name: &str, 
    options: &[String], 
    suppress_already_installed_messages: bool
) -> ShardResult<()> {
    package_operation_with_options(
        PackageType::Cask,
        PackageOperation::Upgrade,
        name,
        options,
        suppress_already_installed_messages
    )
}

/// Helper function to uninstall a package (formula or cask)
pub fn uninstall_package(
    package_type: PackageType,
    name: &str,
    force: bool
) -> ShardResult<()> {
    let type_str = package_type.as_str();
    println!("Uninstalling {}: {}", type_str, style(name).bold());
    
    let result = match package_type {
        PackageType::Formula => get_brew_client().uninstall_formula(name, force),
        PackageType::Cask => get_brew_client().uninstall_cask(name, force),
    };
    
    match result {
        Ok(_) => {
            println!("Successfully uninstalled {}: {}", type_str, style(name).bold());
            Ok(())
        },
        Err(e) => {
            // Check for common error cases
            let error_msg = e.to_string();
            if error_msg.contains("is not installed") {
                println!("{} {} is not installed, skipping", type_str, style(name).bold());
                Ok(())
            } else {
                eprintln!("Error uninstalling {} {}: {}", type_str, name, e);
                Err(e)
            }
        }
    }
}

/// Helper function to add a tap
pub fn add_tap(name: &str) -> ShardResult<()> {
    println!("Adding tap: {}", style(name).bold());
    match get_brew_client().add_tap(name) {
        Ok(_) => {
            println!("Successfully added tap: {}", style(name).bold());
            Ok(())
        },
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("already tapped") {
                println!("Tap {} is already added", style(name).bold());
                Ok(())
            } else {
                eprintln!("Error adding tap {}: {}", name, e);
                Err(e)
            }
        }
    }
}

/// Run Homebrew cleanup
pub fn run_cleanup() -> ShardResult<()> {
    println!("Running final cleanup...");
    match get_brew_client().cleanup(true) {
        Ok(_) => {
            println!("{} Final cleanup completed", style("✓").bold().green());
            Ok(())
        },
        Err(e) => {
            eprintln!("Error cleaning up Homebrew packages: {}", e);
            Err(e)
        }
    }
}

/// Check if a formula is outdated
pub fn is_formula_outdated(formula: &str) -> ShardResult<bool> {
    let outdated_check = Command::new("brew")
        .args(["outdated", "--formula", formula, "--quiet"])
        .output()
        .with_context(|| format!("Failed to check if formula is outdated: {}", formula))?;
    
    let output = String::from_utf8_lossy(&outdated_check.stdout);
    Ok(!output.trim().is_empty())
}

/// Check if a cask is outdated
pub fn is_cask_outdated(cask: &str) -> ShardResult<bool> {
    let outdated_check = Command::new("brew")
        .args(["outdated", "--cask", cask, "--quiet"])
        .output()
        .with_context(|| format!("Failed to check if cask is outdated: {}", cask))?;
    
    let output = String::from_utf8_lossy(&outdated_check.stdout);
    Ok(!output.trim().is_empty())
}

/// Get a list of all currently installed formulae
pub fn get_installed_formulae() -> ShardResult<Vec<String>> {
    get_brew_client().get_installed_formulae()
        .with_context(|| "Failed to get list of installed formulae")
}

/// Get a list of all currently installed casks
pub fn get_installed_casks() -> ShardResult<Vec<String>> {
    get_brew_client().get_installed_casks()
        .with_context(|| "Failed to get list of installed casks")
}

/// Get a list of all currently installed taps
pub fn get_installed_taps() -> ShardResult<Vec<String>> {
    get_brew_client().get_installed_taps()
        .with_context(|| "Failed to get list of installed taps")
}

/// Get a list of all packages installed as dependencies
pub fn get_dependency_packages() -> ShardResult<Vec<String>> {
    get_brew_client().get_dependency_packages()
        .with_context(|| "Failed to get list of dependency packages")
}

/// Get a list of explicitly installed packages (both formulae and casks, excluding dependencies)
pub fn get_all_main_packages() -> ShardResult<(Vec<String>, Vec<String>)> {
    // Get all installed formulae and casks
    let all_formulae = get_installed_formulae()
        .with_context(|| "Failed to get installed formulae for main package listing")?;
    let all_casks = get_installed_casks()
        .with_context(|| "Failed to get installed casks for main package listing")?;
    
    // Get dependency packages
    let deps = get_dependency_packages()
        .with_context(|| "Failed to get dependency packages")?;
    
    // Filter out dependencies from formulae
    let main_formulae: Vec<String> = all_formulae.into_iter()
        .filter(|f| !deps.contains(f))
        .collect();
    
    Ok((main_formulae, all_casks))
}

/// Generic function to perform batch operations on packages
pub fn batch_package_operation<F>(
    package_type: PackageType,
    operation: &str,
    packages: &[String],
    dry_run: bool,
    operation_fn: F
) -> ShardResult<()>
where
    F: FnOnce(&[String]) -> ShardResult<()>,
{
    if packages.is_empty() {
        return Ok(());
    }

    if dry_run {
        println!("Would {} {} {}s", operation, packages.len(), package_type.as_str());
        for package in packages {
            println!("  - {}", style(package).bold());
        }
        return Ok(());
    }
    
    println!("{} {} {}s...", operation, packages.len(), package_type.as_str());
    
    // Show detailed list of packages to install
    for package in packages {
        println!("  → Will {} {}: {}", 
            operation.to_lowercase(), 
            package_type.as_str(), 
            style(package).bold());
    }
    
    // Create a progress bar
    let progress_bar = ProgressBar::new(packages.len() as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-")
    );
    progress_bar.set_message(format!("{}ing {} packages", operation, package_type.as_str()));
    
    // For individual installation, we'll manually handle each package with progress
    if packages.len() == 1 {
        let package = &packages[0];
        progress_bar.set_message(format!("{}ing {}: {}", 
            operation.to_lowercase(), 
            package_type.as_str(), 
            style(package).bold()));
        
        match operation_fn(packages) {
            Ok(_) => {
                progress_bar.finish_with_message(format!("{} {} {} {}",
                    style("✓").bold().green(),
                    operation,
                    package_type.as_str(),
                    style(package).bold()));
                Ok(())
            },
            Err(e) => {
                progress_bar.finish_with_message(format!("{} Failed to {} {}: {}",
                    style("✗").bold().red(),
                    operation.to_lowercase(),
                    package,
                    e));
                Err(e)
            }
        }
    } else {
        // For multiple packages
        match operation_fn(packages) {
            Ok(_) => {
                progress_bar.finish_with_message(format!("{} Successfully {}d {} {}s", 
                    style("✓").bold().green(),
                    operation.to_lowercase(),
                    packages.len(),
                    package_type.as_str()));
                Ok(())
            },
            Err(e) => {
                progress_bar.finish_with_message(format!("{} Error {}ing {} {}s: {}", 
                    style("✗").bold().red(),
                    operation.to_lowercase(),
                    packages.len(),
                    package_type.as_str(),
                    e));
                Err(e)
            }
        }
    }
}

/// Perform a batch install of multiple formulae at once
pub fn batch_install_formulae(formulae: &[String], dry_run: bool) -> ShardResult<()> {
    batch_package_operation(
        PackageType::Formula,
        "Installing",
        formulae,
        dry_run,
        |packages| get_brew_client().batch_install_formulae(packages)
    )
}

/// Perform a batch upgrade of multiple formulae at once
pub fn batch_upgrade_formulae(formulae: &[String], dry_run: bool) -> ShardResult<()> {
    batch_package_operation(
        PackageType::Formula,
        "Upgrading",
        formulae,
        dry_run,
        |packages| get_brew_client().batch_upgrade_formulae(packages)
    )
}

/// Perform a batch install of multiple casks at once
pub fn batch_install_casks(casks: &[String], dry_run: bool) -> ShardResult<()> {
    batch_package_operation(
        PackageType::Cask,
        "Installing",
        casks,
        dry_run,
        |packages| get_brew_client().batch_install_casks(packages)
    )
}

/// Perform a batch upgrade of multiple casks at once
pub fn batch_upgrade_casks(casks: &[String], dry_run: bool) -> ShardResult<()> {
    batch_package_operation(
        PackageType::Cask,
        "Upgrading",
        casks,
        dry_run,
        |packages| get_brew_client().batch_upgrade_casks(packages)
    )
}

/// Result of processing packages: items to install, upgrade, with options, and to uninstall
#[derive(Default)]
pub struct PackageProcessResult {
    pub to_install: Vec<String>,
    pub to_upgrade: Vec<String>,
    pub with_options: Vec<(String, Vec<String>)>,
    pub to_uninstall: Vec<String>,
}

/// Trait for objects that contain package information
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

impl<'a> PackageInfo for &'a Formula {
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

impl<'a> PackageInfo for &'a Cask {
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
}

impl PackageProcessor {
    /// Create a new processor with the given package type and installed packages
    pub fn new(package_type: PackageType, installed_packages: Vec<String>, suppress_messages: bool) -> Self {
        Self {
            package_type,
            installed_packages,
            suppress_messages,
        }
    }
    
    /// Check if a package is installed
    pub fn is_installed(&self, name: &str) -> bool {
        self.installed_packages.contains(&name.to_string())
    }
    
    /// Process a list of packages based on their state
    pub fn process_packages<T>(&self, packages: &[T]) -> ShardResult<PackageProcessResult>
    where 
        T: PackageInfo,
    {
        let mut result = PackageProcessResult::default();
        let pkg_type_str = self.package_type.as_str();
        
        // First pass: categorize packages
        for package in packages {
            let name = package.name();
            let state = package.state();
            let options = package.options();
            let has_options = !options.is_empty();
            let is_installed = self.is_installed(name);
            
            match state {
                PackageState::Absent => {
                    // Should be uninstalled if currently installed
                    if is_installed {
                        result.to_uninstall.push(name.to_string());
                    }
                },
                PackageState::Latest if has_options => {
                    // Has custom options, needs individual processing
                    result.with_options.push((name.to_string(), options.to_vec()));
                },
                PackageState::Latest if is_installed => {
                    // Already installed, needs upgrade
                    result.to_upgrade.push(name.to_string());
                },
                PackageState::Present if has_options => {
                    // Has custom options, needs individual processing
                    if !is_installed {
                        result.with_options.push((name.to_string(), options.to_vec()));
                    } else if !self.suppress_messages {
                        println!("{} already installed: {}", 
                            pkg_type_str, style(name).bold());
                    }
                },
                _ if !is_installed => {
                    // Not installed, needs to be installed
                    result.to_install.push(name.to_string());
                },
                _ if !self.suppress_messages => {
                    // Already installed, no action needed
                    println!("{} already installed: {}", 
                        pkg_type_str, style(name).bold());
                },
                _ => {},
            }
        }
        
        Ok(result)
    }
    
    /// Execute operations on the packages based on the processed results
    pub fn execute_operations(&self, result: &PackageProcessResult, dry_run: bool) -> ShardResult<()> {
        let pkg_type_str = self.package_type.as_str();
        
        // Process installations
        if !result.to_install.is_empty() {
            match self.package_type {
                PackageType::Formula => batch_install_formulae(&result.to_install, dry_run)?,
                PackageType::Cask => batch_install_casks(&result.to_install, dry_run)?,
            }
        }
        
        // Process upgrades
        if !result.to_upgrade.is_empty() {
            match self.package_type {
                PackageType::Formula => batch_upgrade_formulae(&result.to_upgrade, dry_run)?,
                PackageType::Cask => batch_upgrade_casks(&result.to_upgrade, dry_run)?,
            }
        }
        
        // Process packages with options
        if !dry_run {
            for (name, options) in &result.with_options {
                let is_installed = self.is_installed(name);
                
                if is_installed {
                    // Upgrade with options
                    match self.package_type {
                        PackageType::Formula => {
                            upgrade_formula_with_options(name, options, self.suppress_messages)?
                        },
                        PackageType::Cask => {
                            upgrade_cask_with_options(name, options, self.suppress_messages)?
                        },
                    }
                } else {
                    // Install with options
                    match self.package_type {
                        PackageType::Formula => {
                            install_formula_with_options(name, options, self.suppress_messages)?
                        },
                        PackageType::Cask => {
                            install_cask_with_options(name, options, self.suppress_messages)?
                        },
                    }
                }
            }
        } else if !result.with_options.is_empty() {
            // Dry run output for packages with options
            for (name, options) in &result.with_options {
                let is_installed = self.is_installed(name);
                let operation = if is_installed { "upgrade" } else { "install" };
                println!("Would {} {} {} with options: {}", 
                    operation, pkg_type_str, style(name).bold(), 
                    options.join(" "));
            }
        }
        
        // Process uninstallations
        if !result.to_uninstall.is_empty() && !dry_run {
            for name in &result.to_uninstall {
                uninstall_package(self.package_type, name, true)?;
            }
        } else if !result.to_uninstall.is_empty() {
            // Dry run output for uninstallations
            for name in &result.to_uninstall {
                println!("Would uninstall {}: {}", pkg_type_str, style(name).bold());
            }
        }
        
        Ok(())
    }

    /// Create a new processor for formulae
    pub fn for_formulae(suppress_messages: bool) -> ShardResult<Self> {
        Ok(Self {
            package_type: PackageType::Formula,
            installed_packages: get_installed_formulae()?,
            suppress_messages,
        })
    }
    
    /// Create a new processor for casks
    pub fn for_casks(suppress_messages: bool) -> ShardResult<Self> {
        Ok(Self {
            package_type: PackageType::Cask,
            installed_packages: get_installed_casks()?,
            suppress_messages,
        })
    }
    
    /// Get a list of all currently installed formulae
    pub fn get_installed_formulae() -> ShardResult<Vec<String>> {
        get_installed_formulae()
    }

    /// Get a list of all currently installed casks
    pub fn get_installed_casks() -> ShardResult<Vec<String>> {
        get_installed_casks()
    }

    /// Get a list of all currently installed taps
    pub fn get_installed_taps() -> ShardResult<Vec<String>> {
        get_installed_taps()
    }

    /// Get a list of all packages installed as dependencies
    pub fn get_dependency_packages() -> ShardResult<Vec<String>> {
        get_dependency_packages()
    }
    
    /// Get a list of explicitly installed packages (both formulae and casks, excluding dependencies)
    pub fn get_all_main_packages() -> ShardResult<(Vec<String>, Vec<String>)> {
        get_all_main_packages()
    }
    
    /// Uninstall a package using this processor's package type
    pub fn uninstall(&self, name: &str, force: bool) -> ShardResult<()> {
        uninstall_package(self.package_type, name, force)
    }
    
    /// Uninstall a package
    pub fn uninstall_package(package_type: PackageType, name: &str, force: bool) -> ShardResult<()> {
        uninstall_package(package_type, name, force)
    }
}

/// Utility struct for Homebrew operations
pub struct BrewUtils;

impl BrewUtils {
    /// Run Homebrew cleanup
    pub fn run_cleanup() -> ShardResult<()> {
        println!("Running final cleanup...");
        match get_brew_client().cleanup(true) {
            Ok(_) => {
                println!("{} Final cleanup completed", style("✓").bold().green());
                Ok(())
            },
            Err(e) => {
                eprintln!("Error cleaning up Homebrew packages: {}", e);
                Err(e)
            }
        }
    }
    
    /// Add a tap to Homebrew
    pub fn add_tap(name: &str) -> ShardResult<()> {
        println!("Adding tap: {}", style(name).bold());
        match get_brew_client().add_tap(name) {
            Ok(_) => {
                println!("Successfully added tap: {}", style(name).bold());
                Ok(())
            },
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("already tapped") {
                    println!("Tap {} already exists, skipping", style(name).bold());
                    Ok(())
                } else {
                    eprintln!("Error adding tap {}: {}", name, e);
                    Err(e)
                }
            }
        }
    }
}
