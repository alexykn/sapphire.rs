use anyhow::{Context, Result};

/// Bootstrap the system with required dependencies
pub fn bootstrap_system() -> Result<()> {
    // Check if Homebrew is installed
    let homebrew_installed = check_homebrew_installed()?;
    
    if !homebrew_installed {
        install_homebrew()
            .context("Failed to install Homebrew")?;
    }
    
    // Check for required dependencies
    check_dependencies()?;
    
    Ok(())
}

fn check_homebrew_installed() -> Result<bool> {
    // Run a simple command to check if Homebrew is available
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg("command -v brew")
        .output()
        .context("Failed to check for Homebrew")?;
    
    Ok(output.status.success())
}

fn install_homebrew() -> Result<()> {
    tracing::info!("Installing Homebrew...");
    
    // This URL might change - should probably be configurable
    let install_cmd = "bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"";
    
    let status = std::process::Command::new("bash")
        .arg("-c")
        .arg(install_cmd)
        .status()
        .context("Failed to execute Homebrew installation script")?;
    
    if !status.success() {
        anyhow::bail!("Homebrew installation failed");
    }
    
    tracing::info!("Homebrew installed successfully");
    Ok(())
}

fn check_dependencies() -> Result<()> {
    // List of required dependencies
    let dependencies = [
        "git",
        "jq",
    ];
    
    let mut missing_deps = Vec::new();
    
    for dep in dependencies.iter() {
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {}", dep))
            .output()
            .context(format!("Failed to check for {}", dep))?;
        
        if !output.status.success() {
            missing_deps.push(*dep);
        }
    }
    
    if !missing_deps.is_empty() {
        tracing::warn!("Missing dependencies: {}", missing_deps.join(", "));
        // Could automatically install these with Homebrew, but let's just warn for now
    }
    
    Ok(())
}