use thiserror::Error;

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

    #[error("Package manager error: {0}")]
    PackageManager(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type SapphireResult<T> = std::result::Result<T, SapphireError>;