use anyhow::{Result, Context, anyhow};
use std::process::{Command, Child, Stdio};
use std::fmt::Write;
use std::time::{Duration, Instant};
use std::thread;
use std::io::{Read, BufReader, BufRead};
use std::sync::mpsc;
use crate::utils::validate as validation;

/// Homebrew client for interacting with brew CLI
pub struct BrewClient {
    /// Path to the brew executable
    brew_path: String,
    /// Whether to enable debug output
    debug: bool,
    /// Command timeout in seconds (None means no timeout)
    timeout: Option<u64>,
}

impl BrewClient {
    /// Create a new client with the default brew path
    pub fn new() -> Self {
        Self {
            brew_path: "brew".to_string(),
            debug: false,
            timeout: None,
        }
    }
    
    /// Create a new client with a custom brew path
    pub fn with_path(brew_path: String) -> Self {
        Self { 
            brew_path,
            debug: false,
            timeout: None,
        }
    }
    
    /// Enable debug logging
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }
    
    /// Set a timeout for commands in seconds
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout = Some(seconds);
        self
    }
    
    // Added validation methods
    /// Validate a package name (formula or cask)
    fn validate_package_name(&self, name: &str) -> Result<String> {
        validation::validate_package_name(name).map(|s| s.to_string())
    }
    
    /// Validate a tap name
    fn validate_tap_name(&self, name: &str) -> Result<String> {
        validation::validate_tap_name(name).map(|s| s.to_string())
    }
    
    /// Validate command options
    fn validate_options(&self, options: &[String]) -> Result<()> {
        validation::validate_options(options)
    }
    
    /// Validate a search query
    fn validate_search_query(&self, query: &str) -> Result<String> {
        validation::validate_search_query(query).map(|s| s.to_string())
    }
    
    // Private helper methods
    /// Execute a brew command and return its output if successful
    fn execute_brew_command(&self, args: &[&str]) -> Result<std::process::Output> {
        let mut cmd = Command::new(&self.brew_path);
        for arg in args {
            cmd.arg(arg);
        }
        
        if self.debug {
            let cmd_str = format!("{} {}", self.brew_path, args.join(" "));
            eprintln!("Executing: {}", cmd_str);
        }
        
        // If timeout is set, use the timeout approach
        if let Some(timeout_secs) = self.timeout {
            self.execute_with_timeout(&mut cmd, timeout_secs)
        } else {
            // Otherwise use the standard approach
            let output = cmd.output()
                .context(format!("Failed to execute brew command: {:?}", args))?;
                
            self.process_output(&output, args);
            
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("Error executing brew command {:?}: {}", args, stderr);
            }
            
            Ok(output)
        }
    }
    
    /// Process and optionally log command output
    fn process_output(&self, output: &std::process::Output, context: impl std::fmt::Debug) -> bool {
        if self.debug {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            let mut debug_output = String::new();
            if !stdout.is_empty() {
                writeln!(debug_output, "STDOUT:\n{}", stdout).unwrap();
            }
            
            if !stderr.is_empty() {
                writeln!(debug_output, "STDERR:\n{}", stderr).unwrap();
            }
            
            if !debug_output.is_empty() {
                eprintln!("Command output:\n{}", debug_output);
            }
        }
        
        output.status.success()
    }
    
    /// Execute a command with a timeout
    fn execute_with_timeout(&self, cmd: &mut Command, timeout_secs: u64) -> Result<std::process::Output> {
        // Configure the command to capture stdout and stderr
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        // Start the child process
        let mut child = cmd.spawn()
            .context("Failed to spawn command")?;
            
        // Track the start time
        let start = Instant::now();
        let timeout = Duration::from_secs(timeout_secs);
        
        // Poll until complete or timeout
        loop {
            // Check if process completed
            match child.try_wait() {
                Ok(Some(status)) => {
                    // Process finished, collect output
                    let output = self.collect_child_output(child, status)?;
                    return Ok(output);
                }
                Ok(None) => {
                    // Still running, check for timeout
                    if start.elapsed() > timeout {
                        if self.debug {
                            eprintln!("Command timed out after {} seconds", timeout_secs);
                        }
                        
                        // Kill the process
                        let _ = child.kill();
                        anyhow::bail!("Command timed out after {} seconds", timeout_secs);
                    }
                    
                    // Sleep briefly to avoid high CPU usage
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => return Err(anyhow::anyhow!("Error waiting for process: {}", e)),
            }
        }
    }
    
    /// Collect output from a child process
    fn collect_child_output(&self, mut child: Child, status: std::process::ExitStatus) -> Result<std::process::Output> {
        // Read stdout and stderr
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        
        if let Some(mut stdout_pipe) = child.stdout.take() {
            stdout_pipe.read_to_end(&mut stdout)
                .context("Failed to read stdout")?;
        }
        
        if let Some(mut stderr_pipe) = child.stderr.take() {
            stderr_pipe.read_to_end(&mut stderr)
                .context("Failed to read stderr")?;
        }
        
        Ok(std::process::Output {
            status,
            stdout,
            stderr,
        })
    }
    
    /// Execute a brew command with custom arguments and return its output if successful
    fn execute_brew_command_with_args(&self, base_args: &[&str], extra_args: &[String]) -> Result<std::process::Output> {
        // Validate extra arguments for safety
        self.validate_options(extra_args)?;
        
        let mut cmd = Command::new(&self.brew_path);
        
        // Add base arguments
        for arg in base_args {
            cmd.arg(arg);
        }
        
        // Add extra arguments
        for arg in extra_args {
            cmd.arg(arg);
        }
        
        if self.debug {
            let mut cmd_str = format!("{} {}", self.brew_path, base_args.join(" "));
            for arg in extra_args {
                write!(cmd_str, " {}", arg).unwrap();
            }
            eprintln!("Executing: {}", cmd_str);
        }
        
        let cmd_str = format!("{} {}", self.brew_path, base_args.join(" "));
        
        // If timeout is set, use the timeout approach
        let output = if let Some(timeout_secs) = self.timeout {
            self.execute_with_timeout(&mut cmd, timeout_secs)?
        } else {
            // Otherwise use the standard approach
            cmd.output()
                .context(format!("Failed to execute command: {}", cmd_str))?
        };
        
        self.process_output(&output, &cmd_str);
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Error executing command {}: {}", cmd_str, stderr);
        }
        
        Ok(output)
    }
    
    /// Parse command output into a list of strings
    fn parse_list_output(&self, output: std::process::Output) -> Vec<String> {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Add a Homebrew tap
    pub fn add_tap(&self, tap: &str) -> Result<()> {
        // Validate tap name before execution
        let validated_tap = self.validate_tap_name(tap)?;
        
        self.execute_brew_command(&["tap", &validated_tap])?;
        Ok(())
    }
    
    /// Install a Homebrew formula
    pub fn install_formula(&self, formula: &str, options: &[String]) -> Result<()> {
        // Validate formula name before execution
        let validated_formula = self.validate_package_name(formula)?;
        
        self.execute_brew_command_with_args(&["install", &validated_formula], options)?;
        Ok(())
    }
    
    /// Install a Homebrew cask
    pub fn install_cask(&self, cask: &str, options: &[String]) -> Result<()> {
        // Validate cask name before execution
        let validated_cask = self.validate_package_name(cask)?;
        
        self.execute_brew_command_with_args(&["install", "--cask", &validated_cask], options)?;
        Ok(())
    }

    /// Get a list of all currently installed formulae
    pub fn get_installed_formulae(&self) -> Result<Vec<String>> {
        let output = self.execute_brew_command(&["list", "--formula"])?;
        Ok(self.parse_list_output(output))
    }

    /// Get a list of all currently installed casks
    pub fn get_installed_casks(&self) -> Result<Vec<String>> {
        let output = self.execute_brew_command(&["list", "--cask"])?;
        Ok(self.parse_list_output(output))
    }

    /// Get a list of all currently installed taps
    pub fn get_installed_taps(&self) -> Result<Vec<String>> {
        let output = self.execute_brew_command(&["tap"])?;
        Ok(self.parse_list_output(output))
    }

    /// Perform a batch install of multiple formulae at once
    pub fn batch_install_formulae(&self, formulae: &[String]) -> Result<()> {
        if formulae.is_empty() {
            return Ok(());
        }
        
        // Validate all package names
        for formula in formulae {
            self.validate_package_name(formula)?;
        }
        
        // Convert String slice to &str slice for the helper function
        let formulae_refs: Vec<&str> = formulae.iter().map(|s| s.as_str()).collect();
        
        // Create a vec with "install" followed by all formula names
        let mut args = vec!["install"];
        args.extend(formulae_refs.iter());
        
        self.execute_brew_command(&args)?;
        Ok(())
    }

    /// Perform a batch install of multiple casks at once
    pub fn batch_install_casks(&self, casks: &[String]) -> Result<()> {
        if casks.is_empty() {
            return Ok(());
        }
        
        // Validate all package names
        for cask in casks {
            self.validate_package_name(cask)?;
        }
        
        // Convert String slice to &str slice for the helper function
        let cask_refs: Vec<&str> = casks.iter().map(|s| s.as_str()).collect();
        
        // Create a vec with install --cask followed by all cask names
        let mut args = vec!["install", "--cask"];
        args.extend(cask_refs.iter());
        
        self.execute_brew_command(&args)?;
        Ok(())
    }

    /// Perform a batch upgrade of multiple formulae at once
    pub fn batch_upgrade_formulae(&self, formulae: &[String]) -> Result<()> {
        if formulae.is_empty() {
            return Ok(());
        }
        
        // Validate all package names
        for formula in formulae {
            self.validate_package_name(formula)?;
        }
        
        let formulae_refs: Vec<&str> = formulae.iter().map(|s| s.as_str()).collect();
        let mut args = vec!["upgrade"];
        args.extend(formulae_refs.iter());
        
        self.execute_brew_command(&args)?;
        Ok(())
    }

    /// Perform a batch upgrade of multiple casks at once
    pub fn batch_upgrade_casks(&self, casks: &[String]) -> Result<()> {
        if casks.is_empty() {
            return Ok(());
        }
        
        // Validate all package names
        for cask in casks {
            self.validate_package_name(cask)?;
        }
        
        let cask_refs: Vec<&str> = casks.iter().map(|s| s.as_str()).collect();
        let mut args = vec!["upgrade", "--cask"];
        args.extend(cask_refs.iter());
        
        self.execute_brew_command(&args)?;
        Ok(())
    }

    /// Upgrade a formula with custom options
    pub fn upgrade_formula_with_options(&self, formula: &str, options: &[String]) -> Result<()> {
        // Validate formula name and options
        let validated_formula = self.validate_package_name(formula)?;
        
        self.execute_brew_command_with_args(&["upgrade", &validated_formula], options)?;
        Ok(())
    }

    /// Upgrade a cask with custom options
    pub fn upgrade_cask_with_options(&self, cask: &str, options: &[String]) -> Result<()> {
        // Validate cask name and options
        let validated_cask = self.validate_package_name(cask)?;
        
        self.execute_brew_command_with_args(&["upgrade", "--cask", &validated_cask], options)?;
        Ok(())
    }

    /// Uninstall a formula
    pub fn uninstall_formula(&self, formula: &str, force: bool) -> Result<()> {
        // Validate formula name
        let validated_formula = self.validate_package_name(formula)?;
        
        let mut args = vec!["uninstall", "--formula", &validated_formula];
        
        if force {
            args.push("--force");
        }
        
        self.execute_brew_command(&args)?;
        Ok(())
    }

    /// Uninstall a cask
    pub fn uninstall_cask(&self, cask: &str, force: bool) -> Result<()> {
        // Validate cask name
        let validated_cask = self.validate_package_name(cask)?;
        
        let mut args = vec!["uninstall", "--cask", &validated_cask];
        
        if force {
            args.push("--force");
        }
        
        self.execute_brew_command(&args)?;
        Ok(())
    }

    /// Get a list of all packages installed as dependencies
    pub fn get_dependency_packages(&self) -> Result<Vec<String>> {
        let output = self.execute_brew_command(&["list", "--installed-as-dependency"])?;
        Ok(self.parse_list_output(output))
    }

    /// Run cleanup
    pub fn cleanup(&self, prune_all: bool) -> Result<()> {
        let mut args = vec!["cleanup"];
        
        if prune_all {
            args.push("--prune=all");
        }
        
        self.execute_brew_command(&args)?;
        Ok(())
    }
    
    /// Search for packages
    pub fn search(&self, query: &str, formula_only: bool, cask_only: bool) -> Result<Vec<String>> {
        // Validate search query
        let validated_query = self.validate_search_query(query)?;
        
        let mut args = vec!["search"];
        
        if formula_only {
            args.push("--formula");
        } else if cask_only {
            args.push("--cask");
        }
        
        args.push(&validated_query);
        
        let output = self.execute_brew_command(&args)?;
        Ok(self.parse_list_output(output))
    }
}