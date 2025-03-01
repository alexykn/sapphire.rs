# CLAUDE.md - Sapphire Project Assistant Guide

## Build Commands
```bash
cargo build                  # Build project
cargo run                    # Run application
cargo test                   # Run all tests
cargo test -- --test-threads=1 <test_name>  # Run specific test
cargo check                  # Check code without building
cargo clippy                 # Run linter
cargo fmt                    # Format code
```

## Code Style Guidelines
- **Naming**: snake_case for variables/functions, CamelCase for types/traits/enums
- **Error Handling**: Use Result<T, E> with context via anyhow, propagate with ?
- **Pattern Matching**: Prefer match over if/else, especially for enums and Result/Option
- **Documentation**: Docstrings for all pub functions with Args/Returns sections
- **Testing**: Write behavior-based tests in a #[cfg(test)] module
- **Functions**: Keep focused and under 30 lines, use structs for >3 parameters
- **Organization**: Group related functionality into modules, minimize dependencies

## Project Structure
Sapphire is a declarative system management suite with three components:
- **Sapphire**: Core application for system setup and management
- **Shard**: Declarative package manager built on Homebrew
- **Fragment**: Configuration system using YAML fragments