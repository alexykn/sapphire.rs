//! Input validation utilities for Homebrew operations.
//!
//! This module provides functions to validate user inputs for security,
//! particularly to prevent command injection vulnerabilities. It defines
//! regular expressions that restrict inputs to safe patterns and provides
//! validation functions used throughout the Homebrew module.
//!
//! # Security
//!
//! All user-provided inputs (package names, tap names, options, etc.) must be
//! validated using the appropriate function from this module before being used
//! in command execution to prevent potential command injection attacks.

/// Utilities for validating user input for security
use crate::utils::{ShardResult, ShardError};
use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    // Valid Homebrew package/formula/cask name regex
    // Allows alphanumeric characters, dashes, underscores, dots, plus signs, and at signs (for versioned packages like openssl@3)
    // More restrictive than what Homebrew technically allows, but catches most command injection attempts
    static ref PACKAGE_NAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9][a-zA-Z0-9_\-\.+@]*$").unwrap();
    
    // Valid Homebrew tap name regex (e.g., "user/repo" or "homebrew/core")
    static ref TAP_NAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_\-]+/[a-zA-Z0-9_\-]+$").unwrap();
    
    // Valid option regex - more permissive, but still restricted
    static ref OPTION_REGEX: Regex = Regex::new(r"^--?[a-zA-Z0-9_\-]+(=[a-zA-Z0-9_\-\.+/]+)?$").unwrap();
}

/// Validate a Homebrew package name (formula or cask)
pub fn validate_package_name(name: &str) -> ShardResult<&str> {
    if name.is_empty() {
        return Err(ShardError::ValidationError("Package name cannot be empty".to_string()));
    }
    
    if !PACKAGE_NAME_REGEX.is_match(name) {
        return Err(ShardError::ValidationError(
            format!("Invalid package name format: '{}'. Names must contain only letters, numbers, dots, dashes, underscores, plus signs, and at signs (@), and must start with a letter or number.", name)
        ));
    }
    
    Ok(name)
}

/// Validate a Homebrew tap name
pub fn validate_tap_name(name: &str) -> ShardResult<&str> {
    if name.is_empty() {
        return Err(ShardError::ValidationError("Tap name cannot be empty".to_string()));
    }
    
    if !TAP_NAME_REGEX.is_match(name) {
        return Err(ShardError::ValidationError(
            format!("Invalid tap name format: '{}'. Names must be in the format 'user/repo'", name)
        ));
    }
    
    Ok(name)
}

/// Validate a Homebrew command option
pub fn validate_option(option: &str) -> ShardResult<&str> {
    if option.is_empty() {
        return Err(ShardError::ValidationError("Option cannot be empty".to_string()));
    }
    
    if !OPTION_REGEX.is_match(option) {
        return Err(ShardError::ValidationError(
            format!("Invalid option format: '{}'. Options must start with - or -- followed by alphanumeric characters, and may include an = with a value", option)
        ));
    }
    
    Ok(option)
}

/// Validate a search query - slightly more permissive than package names
pub fn validate_search_query(query: &str) -> ShardResult<&str> {
    if query.is_empty() {
        return Err(ShardError::ValidationError("Search query cannot be empty".to_string()));
    }
    
    // Allow spaces and some special characters for search, but still block obvious shell injection
    if query.contains(';') || query.contains('&') || query.contains('|') || 
       query.contains('<') || query.contains('>') || query.contains('`') {
        return Err(ShardError::ValidationError(
            format!("Invalid search query: '{}'. Query contains prohibited characters", query)
        ));
    }
    
    Ok(query)
}

/// Validate a vector of options
pub fn validate_options(options: &[String]) -> ShardResult<()> {
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