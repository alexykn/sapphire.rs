# Rust Coding Guidelines for Sapphire

## Core Principles

- **Clear over clever**: Prioritize readability and clarity above all else
- **Self-documenting code**: Code should explain itself through naming and structure
- **Minimalist documentation**: Document only what's necessary, not what's obvious
- **Fail fast and explicitly**: Error handling should be comprehensive but clear

## Naming Conventions

- **Be descriptive**: Use long, self-explanatory names over short, cryptic ones
- **Use snake_case**: For variables, functions, modules, and packages
- **Use CamelCase**: For types, traits, and enums
- **Be specific**: `create_user_account()` is better than `create()`
- **Use domain language**: Names should reflect the business domain

## Functions

- **Single responsibility**: Each function should do one thing well
- **Parameter limits**: When a function needs more than 3-4 parameters, use a struct
- **Builder pattern**: For complex object creation with many optional parameters
- **Return Result<T, E>**: For operations that can fail; avoid panicking
- **Docstrings**: Required for all `pub` functions that are used across modules

```rust
/// Creates a new package manifest from the provided specification.
/// 
/// # Arguments
/// 
/// * `spec` - The package specification containing all necessary details
/// * `path` - Optional path where the manifest should be saved
/// 
/// # Returns
/// 
/// A Result containing the created Manifest or an error if creation failed
pub fn create_package_manifest(spec: PackageSpec, path: Option<PathBuf>) -> Result<Manifest, ManifestError> {
    // Implementation
}
```

## Pattern Matching

- **Prefer match over if/else**: Especially for enums and Option/Result types
- **Exhaustive matching**: Always handle all possible cases
- **Use guards**: For complex conditions within match arms
- **Destructure in patterns**: Extract needed values directly in the pattern

```rust
// Prefer this:
match package_status {
    Status::Installed { version } if version < min_version => upgrade_package(package),
    Status::Installed { .. } => Ok(()),
    Status::NotInstalled => install_package(package),
    Status::Failed(err) => Err(err.into()),
}

// Over this:
if package_status.is_installed() {
    if package_status.version() < min_version {
        upgrade_package(package)
    } else {
        Ok(())
    }
} else if package_status.is_not_installed() {
    install_package(package)
} else {
    Err(package_status.error().into())
}
```

## Error Handling

- **Use anyhow for applications**: For flexible, context-rich errors
- **Create custom errors for libraries**: Define specific error types for APIs
- **Add context to errors**: Use `.context()` or `.with_context()` to add information
- **Propagate errors with `?`**: Use the question mark operator for concise error propagation

```rust
fn apply_fragment(fragment: &Fragment) -> Result<(), anyhow::Error> {
    let config = parse_config(&fragment.config_path)
        .context("Failed to parse fragment configuration")?;
    
    validate_fragment_config(&config)
        .context("Fragment configuration validation failed")?;
    
    // Rest of implementation
    Ok(())
}
```

## Comments

- **Minimize inline comments**: If you need a comment, consider renaming or refactoring
- **Use TODO/FIXME**: Mark unfinished or problematic code for future attention
- **Comment rationale**: Explain "why", not "what" or "how"
- **Comment non-obvious code**: Explain code that can't be made self-explanatory

```rust
// Good comment (explains why):
// We're using a custom parser here because the standard parser doesn't handle
// legacy configuration format with unescaped quotes

// Bad comment (explains what, which should be clear from the code):
// Loop through packages and install each one
```

## Code Organization

- **Small functions**: Keep functions focused and under 30 lines when possible
- **Module organization**: Group related functionality into modules
- **Minimize dependencies**: Only import what you need
- **Hide implementation details**: Use `pub(crate)` and private modules

## Testing

- **Write tests for behavior**: Test what functions do, not how they do it
- **Descriptive test names**: `test_add_package_creates_new_entry_in_manifest()`
- **Use test modules**: Place tests in a module with the `#[cfg(test)]` attribute
- **Mock external services**: Use traits + mock implementations for testing

## Project Structure

- **Feature flags**: Use Cargo features to make functionality optional
- **Separation of concerns**: Keep CLI, business logic, and I/O separate
- **Internal APIs**: Create clear boundaries between components
- **Follow standard paths**: Use Rust community conventions for project layout