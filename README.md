# Sapphire

> **⚠️ IMPORTANT: This is a work-in-progress project that is NOT ready for general use.**

Sapphire is a comprehensive system management suite for macOS that aims to provide declarative configuration management without relying on Nix.

## Project Status

This project is in very early development:

- **Shard**: The declarative package manager component is the only part that's somewhat functional
- **Sapphire Core**: Under development
- **Fragment**: Under development
- **Lapidary**: Future server component (not started)

## What is Sapphire?

Sapphire aims to be a complete solution for macOS system management with three main components:

1. **Sapphire Core**: Initial system setup, application management, and orchestration
2. **Shard**: Declarative package management built on top of Homebrew
3. **Fragment**: Configuration management system using YAML fragments

## Roadmap

The project is being developed in phases:

### Current Focus
- Refining the Shard package manager functionality
- Setting up the foundation for the Fragment configuration system

### Future Development
- Complete Fragment implementation for configuration management
- Develop the core Sapphire application
- Create Lua extension system for custom configuration providers
- Build Lapidary server component for centralized management

## Lapidary (Future)

Lapidary will be the server-side component of Sapphire, enabling centralized management of macOS systems in enterprise environments. It's designed with:

- Centralized configuration and package management
- Security policy enforcement
- User-specific customizations within defined boundaries
- Package allowlists/blocklists

## Contributions

As this project is still in the planning/early development stages, we're not actively seeking contributions yet. Feel free to star the repo if you're interested in following our progress.

## License

This project is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0). See the [LICENSE](LICENSE) file for details.
