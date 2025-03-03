use anyhow::Result;
use std::path::Path;
use sapphire_core::utils::file_system as fs;
use crate::parser::{Fragment, FragmentType};
use serde_yaml::{Mapping, Value};

/// Initialize a new fragment file
pub fn init<P: AsRef<Path>>(fragment_type: &str, path: P) -> Result<()> {
    let path = path.as_ref();
    
    // Ensure the parent directory exists
    if let Some(parent) = path.parent() {
        fs::ensure_dir_exists(parent)?;
    }
    
    // Parse fragment type
    let fragment_type = match fragment_type.to_lowercase().as_str() {
        "dotfiles" => FragmentType::Dotfiles,
        "system" => FragmentType::System,
        "network" => FragmentType::Network,
        "custom" => FragmentType::Custom,
        _ => anyhow::bail!("Invalid fragment type: {}. Must be one of: dotfiles, system, network, custom", fragment_type),
    };
    
    // Create path with extension if needed
    let file_path = if path.is_dir() {
        let filename = format!("{}.toml", fragment_type.to_string().to_lowercase());
        path.join(filename)
    } else {
        path.to_path_buf()
    };
    
    // Check if the fragment already exists
    if fs::path_exists(&file_path) {
        anyhow::bail!("Fragment already exists: {}", file_path.display());
    }
    
    // Create fragment content based on type
    let (description, content) = create_template_content(&fragment_type);
    
    // Clone the fragment_type for later use (inexpensive for enums)
    let fragment_type_for_log = fragment_type.clone();
    
    // Create the fragment
    let fragment = Fragment {
        fragment_type,
        description,
        content: Value::Mapping(content),
    };
    
    // Save the fragment
    fragment.to_file(&file_path)?;
    
    tracing::info!("Created new {} fragment at: {}", fragment_type_for_log.to_string().to_lowercase(), file_path.display());
    
    Ok(())
}

/// Create template content for a fragment based on its type
fn create_template_content(fragment_type: &FragmentType) -> (String, Mapping) {
    let mut content = Mapping::new();
    
    match fragment_type {
        FragmentType::Dotfiles => {
            // Create a template for dotfiles
            let description = "Configuration files for terminal applications".to_string();
            
            let mut files = Vec::new();
            let file1 = make_mapping([
                ("source", "~/.sapphire/dotfiles/zshrc"),
                ("target", "~/.zshrc"),
                ("backup", "true"),
            ]);
            files.push(Value::Mapping(file1));
            
            let file2 = make_mapping([
                ("source", "~/.sapphire/dotfiles/vimrc"),
                ("target", "~/.vimrc"),
                ("backup", "true"),
            ]);
            files.push(Value::Mapping(file2));
            
            content.insert(Value::String("files".to_string()), Value::Sequence(files));
            
            let mut dirs = Vec::new();
            let dir1 = make_mapping([
                ("source", "~/.sapphire/dotfiles/config/nvim"),
                ("target", "~/.config/nvim"),
                ("backup", "true"),
            ]);
            dirs.push(Value::Mapping(dir1));
            
            content.insert(Value::String("directories".to_string()), Value::Sequence(dirs));
            
            (description, content)
        },
        FragmentType::System => {
            // Create a template for system preferences
            let description = "macOS system preferences".to_string();
            
            let mut prefs = Vec::new();
            let pref1 = make_mapping([
                ("domain", "com.apple.dock"),
                ("key", "autohide"),
                ("value_type", "bool"),
                ("value", "true"),
            ]);
            prefs.push(Value::Mapping(pref1));
            
            let pref2 = make_mapping([
                ("domain", "NSGlobalDomain"),
                ("key", "AppleShowAllExtensions"),
                ("value_type", "bool"),
                ("value", "true"),
            ]);
            prefs.push(Value::Mapping(pref2));
            
            content.insert(Value::String("preferences".to_string()), Value::Sequence(prefs));
            
            (description, content)
        },
        FragmentType::Network => {
            // Create a template for network configuration
            let description = "Network configuration settings".to_string();
            
            let mut networks = Vec::new();
            let network1 = make_mapping([
                ("name", "Home Network"),
                ("type", "wifi"),
                ("ssid", "HomeWiFi"),
                ("priority", "10"),
            ]);
            networks.push(Value::Mapping(network1));
            
            content.insert(Value::String("networks".to_string()), Value::Sequence(networks));
            
            let mut proxy = Mapping::new();
            proxy.insert(Value::String("enabled".to_string()), Value::Bool(false));
            proxy.insert(Value::String("server".to_string()), Value::String("proxy.example.com".to_string()));
            proxy.insert(Value::String("port".to_string()), Value::Number(8080.into()));
            
            content.insert(Value::String("proxy".to_string()), Value::Mapping(proxy));
            
            (description, content)
        },
        FragmentType::Custom => {
            // Create a template for custom script
            let description = "Custom configuration using external script".to_string();
            
            content.insert(Value::String("script_path".to_string()), 
                         Value::String("~/.sapphire/scripts/custom.lua".to_string()));
            
            let mut params = Mapping::new();
            params.insert(Value::String("config_dir".to_string()), 
                        Value::String("~/.config".to_string()));
            params.insert(Value::String("app_name".to_string()), 
                        Value::String("my-app".to_string()));
            
            content.insert(Value::String("parameters".to_string()), Value::Mapping(params));
            
            (description, content)
        },
    }
}

/// Helper to create a mapping from key-value pairs
fn make_mapping<const N: usize>(pairs: [(&str, &str); N]) -> Mapping {
    let mut map = Mapping::new();
    for (key, value) in pairs.iter() {
        let yaml_value = match *value {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            v if v.parse::<i64>().is_ok() => {
                Value::Number(v.parse::<i64>().unwrap().into())
            },
            v if v.parse::<f64>().is_ok() => {
                Value::Number(serde_yaml::Number::from(v.parse::<f64>().unwrap()))
            },
            _ => Value::String(value.to_string()),
        };
        map.insert(Value::String(key.to_string()), yaml_value);
    }
    map
}