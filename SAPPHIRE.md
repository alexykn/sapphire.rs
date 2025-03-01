# Sapphire Project Plan

## Overview
Sapphire is a comprehensive system management suite for macOS that provides declarative configuration management without relying on Nix. It consists of three main components:

- **Sapphire**: Core application for initial setup and managing the overall system
- **Shard**: Declarative package manager built on top of Homebrew
- **Fragment**: Configuration management system using YAML fragments for system configuration

## Project Structure

```
sapphire/
├── Cargo.toml
├── src/
│   ├── main.rs               # Entry point with command routing
│   ├── cli/                  # CLI interface using clap
│   │   ├── mod.rs
│   │   ├── sapphire_cli.rs   # Main application CLI
│   │   ├── shard_cli.rs      # Package manager CLI
│   │   └── fragment_cli.rs   # Config manager CLI
│   ├── core/                 # Shared core functionality
│   │   ├── mod.rs
│   │   ├── config.rs         # Configuration management
│   │   ├── paths.rs          # Path management
│   │   └── utils.rs          # Shared utilities
│   ├── sapphire/            # Core application functionality
│   │   ├── mod.rs
│   │   ├── setup.rs          # First-run setup
│   │   ├── bootstrap.rs      # System bootstrapping
│   │   └── manager.rs        # Application management
│   ├── shard/               # Package management
│   │   ├── mod.rs
│   │   ├── brew_client.rs    # Homebrew client
│   │   ├── cask_client.rs    # Homebrew Cask client
│   │   ├── declarative.rs    # Declarative package management
│   │   ├── manifest.rs       # Package manifest handling
│   │   └── operations.rs     # Package operations
│   ├── fragment/            # Config management system
│   │   ├── mod.rs
│   │   ├── parser.rs         # YAML parser
│   │   ├── engine.rs         # Execution engine
│   │   ├── validators.rs     # Fragment validation
│   │   ├── providers/        # Configuration providers
│   │   │   ├── mod.rs
│   │   │   ├── dotfiles.rs   # Dotfile management
│   │   │   ├── system.rs     # System settings
│   │   │   ├── network.rs    # Network configuration
│   │   │   ├── containers.rs # Container management
│   │   │   └── custom.rs     # Custom provider support
│   │   └── extensions/       # Extension system
│   │       ├── mod.rs
│   │       ├── lua_runtime.rs # Lua script execution
│   │       └── registry.rs   # Extension registry
│   └── utils/               # Utility functions
│       ├── mod.rs
│       ├── exec.rs          # Command execution
│       ├── fs.rs            # File system operations
│       ├── templates.rs     # Template rendering
│       └── logging.rs       # Logging utilities
├── config/                  # Default configurations
│   ├── default_fragments/   # Default system fragments
│   ├── shard_templates/     # Package manifest templates
│   └── sapphire.toml        # Main configuration file
└── examples/                # Example configurations
    ├── fragments/          # Example fragments
    │   ├── dotfiles.yaml
    │   ├── containers.yaml
    │   └── system.yaml
    ├── scripts/            # Example extension scripts
    │   ├── setup_dev.lua
    │   └── backup.lua
    └── manifests/          # Example package manifests
        ├── system_apps.yaml
        └── user_apps.yaml
```

## Component Details

### 1. Sapphire Core

**Purpose:** Initial system setup, application management, and orchestration

**Key Features:**
- First-run setup with dependency installation
- Mode detection (local vs. managed - foundation for future server integration)
- Directory structure creation
- Configuration initialization
- Integrated CLI access to Shard and Fragment
- Modern and appealing user interface mainly using cliclack and adding extra formating with console-rs when needed

**Configuration Structure:**
```
~/.sapphire/
├── config.toml              # Core configuration
├── fragments/               # Fragment configurations
│   ├── system/              # System fragments
│   └── user/                # User fragments
├── scripts/                 # Extension scripts (Lua)
└── manifests/               # Package manifests
    ├── system_apps.yaml     # System packages (protected)
    └── user_apps.yaml       # User packages (modifiable)
```

### 2. Shard

**Purpose:** Declarative package management using Homebrew

**Key Features:**
- Full Homebrew/Cask wrapper
- Declarative package management via YAML manifests
- Package verification
- Package synchronization (apply, diff)
- Package search with rich metadata
- Package locking (system apps protection)

**Manifest Structure:**
```yaml
# system_apps.yaml or user_apps.yaml
metadata:
  description: "Core system applications"
  protected: true  # Only for system_apps.yaml
  
formulas:
  - name: git
    version: "latest"
    options: []
    
casks:
  - name: firefox
    version: "latest"
    options: ["no-quarantine"]

taps:
  - name: homebrew/cask-fonts
```

### 3. Fragment

**Purpose:** Configuration management using declarative fragments

**Key Features:**
- YAML-based configuration fragments
- Multiple provider types (dotfiles, system, network, etc.)
- Diff detection
- Backup and restore
- Idempotent operations
- Extension system for custom providers via external Lua scripts

**Fragment Structure:**
```yaml
# Example: dotfiles.yaml
type: dotfiles
description: "Terminal configuration files"

files:
  - source: "~/.sapphire/dotfiles/zshrc"
    target: "~/.zshrc"
    backup: true
    
  - source: "~/.sapphire/dotfiles/tmux.conf"
    target: "~/.tmux.conf"
    
directories:
  - source: "~/.sapphire/dotfiles/nvim"
    target: "~/.config/nvim"
    backup: true
```

```yaml
# Example: system.yaml
type: system
description: "macOS system preferences"

preferences:
  - domain: "com.apple.dock"
    key: "autohide"
    type: "bool"
    value: true
    
  - domain: "NSGlobalDomain"
    key: "AppleShowAllExtensions"
    type: "bool"
    value: true
```

**Custom Fragment with Lua Extension:**
```yaml
# Example: custom.yaml
type: custom
description: "Custom setup using external Lua script"

script_path: "~/.sapphire/scripts/setup_dev.lua"
parameters:
  config_dir: "~/.config"
  app_name: "custom-app"
  version: "1.2.0"
```

## Extension System

The extension system for Fragment will use external Lua scripts as the preferred method:

1. External script files are preferred over inline scripts
2. Scripts are stored in `~/.sapphire/scripts/` directory
3. Fragment can reference scripts by path
4. Parameters can be passed from fragment to script
5. Inline scripts are supported but generate warnings

**Example Lua Extension Script:**
```lua
-- ~/.sapphire/scripts/setup_dev.lua
function setup(params)
  local config_dir = params.config_dir or "~/.config"
  local app_name = params.app_name or "default-app"
  
  os.execute(string.format("mkdir -p %s/%s", config_dir, app_name))
  -- Additional setup logic
  return true
end

function validate(params)
  local config_dir = params.config_dir or "~/.config"
  local app_name = params.app_name or "default-app"
  
  return sapphire.path_exists(string.format("%s/%s", config_dir, app_name))
end

function rollback(params)
  local config_dir = params.config_dir or "~/.config"
  local app_name = params.app_name or "default-app"
  
  os.execute(string.format("rm -rf %s/%s", config_dir, app_name))
  return true
end
```

## Implementation Plan

### Phase 1: Foundation
1. Set up project structure and base modules
2. Implement core configuration management
3. Create basic CLI interfaces for all components
4. Implement path management and utilities

### Phase 2: Shard Implementation
1. Develop Homebrew client integration
2. Create manifest parser and validator
3. Implement package operations (install, remove, update)
4. Add declarative management features
5. Implement system vs. user package separation

### Phase 3: Fragment Implementation
1. Develop YAML fragment parser
2. Create base provider system
3. Implement common providers (dotfiles, system, network)
4. Add backup and restore functionality
5. Create diff detection system

### Phase 4: Lua Extension System
1. Implement Lua runtime integration
2. Create API for Lua scripts to access system functions
3. Develop script loader and executor
4. Add parameter passing from fragments to scripts
5. Implement validation and error handling

### Phase 5: Sapphire Core
1. Implement first-run setup
2. Add dependency management
3. Create directory structure initialization
4. Implement mode detection (local/unmanaged)
5. Add integration between components

## Future Server Integration (Lapidary)

The local implementation is designed with future server integration in mind, focusing on the requirements for the managed mode:

1. Server-side management of fragments (global rays)
2. User fragment backup and synchronization
3. Server-managed overrides for user settings
4. Package allowlists/blocklists
5. GRPC-based communication between client and server

Lapidary will be addressed in a separate detailed plan.
