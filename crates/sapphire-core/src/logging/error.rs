use thiserror::Error;

/// Common error types for Sapphire tools
#[derive(Error, Debug)]
pub enum SapphireError {
    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parsing error: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    #[error("System operation failed: {0}")]
    SystemOperation(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Result type alias for Sapphire operations
pub type SapphireResult<T> = std::result::Result<T, SapphireError>; 