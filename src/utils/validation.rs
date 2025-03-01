/// Utilities for validating user input for security
use anyhow::{bail, Result};
use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    // Valid Homebrew package/formula/cask name regex
    // Allows alphanumeric characters, dashes, underscores, dots, and pluses
    // More restrictive than what Homebrew technically allows, but catches most command injection attempts
    static ref PACKAGE_NAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9][a-zA-Z0-9_\-\.+]*$").unwrap();
    
    // Valid Homebrew tap name regex (e.g., "user/repo" or "homebrew/core")
    static ref TAP_NAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_\-]+/[a-zA-Z0-9_\-]+$").unwrap();
    
    // Valid option regex - more permissive, but still restricted
    static ref OPTION_REGEX: Regex = Regex::new(r"^--?[a-zA-Z0-9_\-]+(=[a-zA-Z0-9_\-\.+/]+)?$").unwrap();
}

/// Validate a Homebrew package name (formula or cask)
pub fn validate_package_name(name: &str) -> Result<&str> {
    if name.is_empty() {
        bail!("Package name cannot be empty");
    }
    
    if !PACKAGE_NAME_REGEX.is_match(name) {
        bail!("Invalid package name format: '{}'. Names must contain only letters, numbers, dots, dashes, underscores, and plus signs, and must start with a letter or number.", name);
    }
    
    Ok(name)
}

/// Validate a Homebrew tap name
pub fn validate_tap_name(name: &str) -> Result<&str> {
    if name.is_empty() {
        bail!("Tap name cannot be empty");
    }
    
    if !TAP_NAME_REGEX.is_match(name) {
        bail!("Invalid tap name format: '{}'. Names must be in the format 'user/repo'", name);
    }
    
    Ok(name)
}

/// Validate a Homebrew command option
pub fn validate_option(option: &str) -> Result<&str> {
    if option.is_empty() {
        bail!("Option cannot be empty");
    }
    
    if !OPTION_REGEX.is_match(option) {
        bail!("Invalid option format: '{}'. Options must start with - or -- followed by alphanumeric characters, and may include an = with a value", option);
    }
    
    Ok(option)
}

/// Validate a search query - slightly more permissive than package names
pub fn validate_search_query(query: &str) -> Result<&str> {
    if query.is_empty() {
        bail!("Search query cannot be empty");
    }
    
    // Allow spaces and some special characters for search, but still block obvious shell injection
    if query.contains(';') || query.contains('&') || query.contains('|') || 
       query.contains('<') || query.contains('>') || query.contains('`') {
        bail!("Invalid search query: '{}'. Query contains prohibited characters", query);
    }
    
    Ok(query)
}

/// Validate a vector of options
pub fn validate_options(options: &[String]) -> Result<()> {
    for option in options {
        validate_option(option)?;
    }
    
    Ok(())
}

/// Test if a string is a valid package name without generating errors
pub fn is_valid_package_name(name: &str) -> bool {
    !name.is_empty() && PACKAGE_NAME_REGEX.is_match(name)
}

/// Test if a string is a valid tap name without generating errors
pub fn is_valid_tap_name(name: &str) -> bool {
    !name.is_empty() && TAP_NAME_REGEX.is_match(name)
} 