# Lapidary - Server Management for Sapphire

## Overview

Lapidary will be the server-side component for Sapphire, enabling central management of macOS systems for enterprise environments, with a focus on ISO27001 certification compliance. It will provide:

1. Centralized management of configurations and packages
2. Override mechanisms for enforcing security policies
3. User-specific customizations within allowed boundaries
4. Package allowlists/blocklists for controlled software deployment
5. Backup and versioning of user configurations

## Key Concepts

- **Global Rays**: Server-managed fragments that apply to all systems
- **User Rays**: User-specific fragments for personalization
- **Server Overwrites**: Admin-defined overwrites that take precedence
- **Permissive Mode**: Blocklist-based package control
- **Restricted Mode**: Allowlist-based package control

## Server Architecture

The Lapidary server will operate as a gRPC service with a management interface, handling requests from Sapphire clients in managed mode. The server manages three key areas:

1. **Configuration Management**: Fragment rays and their distribution
2. **Package Control**: Allowlists/blocklists and package approvals
3. **User Management**: User-specific settings and overwrites

## Implementation Notes

This is a future phase of the project to be detailed after the local Sapphire implementation is complete. The current Sapphire design includes the necessary foundations for Lapidary integration through:

1. Mode detection (local vs. managed)
2. Clear separation of system and user configurations
3. Override mechanisms in the fragment engine
4. Permission controls in package management

## Next Steps

1. Complete the local Sapphire implementation
2. Define detailed requirements for enterprise management
3. Design the server-side API and storage model
4. Develop the gRPC service definition
5. Implement the management interface