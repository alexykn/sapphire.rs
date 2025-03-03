//! Core functionality for executing Homebrew commands.
//!
//! This module provides low-level command execution capabilities for the Homebrew CLI.
//! It handles timeouts, process management, and output parsing, but does not perform
//! input validation. Callers are responsible for validating all inputs before passing
//! them to methods in this module.

use crate::utils::ShardResult;
use anyhow::Context;
use std::process::{Command, Child, Stdio};
use std::fmt::Write;
use std::time::{Duration, Instant};
use std::thread;
use std::io::Read;

/// Core functionality for executing brew commands
#[derive(Clone)]
pub struct BrewCore {
    /// Path to the brew executable
    brew_path: String,
    /// Whether to enable debug output
    debug: bool,
    /// Command timeout in seconds (None means no timeout)
    timeout: Option<u64>,
}

impl BrewCore {
    /// Create a new core with the default brew path
    pub fn new() -> Self {
        Self {
            brew_path: "brew".to_string(),
            debug: false,
            timeout: None,
        }
    }
    
    /// Create a new core with a custom brew path
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
    
    /// Execute a brew command and return its output if successful
    pub fn execute_brew_command(&self, args: &[&str]) -> ShardResult<std::process::Output> {
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
                return Err(crate::utils::ShardError::BrewError(
                    format!("Error executing brew command {:?}: {}", args, stderr)
                ));
            }
            
            Ok(output)
        }
    }
    
    /// Process and optionally log command output
    pub fn process_output(&self, output: &std::process::Output, _context: impl std::fmt::Debug) -> bool {
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
    pub fn execute_with_timeout(&self, cmd: &mut Command, timeout_secs: u64) -> ShardResult<std::process::Output> {
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
                        return Err(crate::utils::ShardError::BrewError(
                            format!("Command timed out after {} seconds", timeout_secs)
                        ));
                    }
                    
                    // Sleep briefly to avoid high CPU usage
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => return Err(crate::utils::ShardError::BrewError(
                    format!("Error waiting for process: {}", e)
                )),
            }
        }
    }
    
    /// Collect output from a child process
    pub fn collect_child_output(&self, mut child: Child, status: std::process::ExitStatus) -> ShardResult<std::process::Output> {
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
    /// 
    /// # Security
    /// 
    /// IMPORTANT: This method assumes all inputs (base_args and extra_args) have been
    /// properly validated by the caller. Unvalidated user input should never be passed
    /// directly to this method as it could lead to command injection vulnerabilities.
    pub fn execute_brew_command_with_args(&self, base_args: &[&str], extra_args: &[&str]) -> ShardResult<std::process::Output> {
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
            return Err(crate::utils::ShardError::BrewError(
                format!("Error executing command {}: {}", cmd_str, stderr)
            ));
        }
        
        Ok(output)
    }
    
    /// Parse command output into a list of strings
    pub fn parse_list_output(&self, output: std::process::Output) -> Vec<String> {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

/// Utility function to get a default brew core instance
pub fn get_core() -> BrewCore {
    BrewCore::new()
} 