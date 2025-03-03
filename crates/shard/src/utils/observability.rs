use std::fmt;
use std::error::Error as StdError;
use std::io;
use std::path::PathBuf;
use thiserror::Error;
use tracing::{debug, error, info, trace, warn, Level};
use tracing_subscriber::{FmtSubscriber, EnvFilter};
use console::style;
use std::sync::Once;

// Static to ensure we only initialize logging once
static INIT_LOGGER: Once = Once::new();

//-------------------------------------------------------------------------------
// Error Handling
//-------------------------------------------------------------------------------

/// Centralized error type for Shard operations 
#[derive(Error, Debug)]
pub enum ShardError {
    #[error("Shard '{0}' not found")]
    NotFound(String),
    
    #[error("Invalid shard name: {0}")]
    InvalidName(String),
    
    #[error("Shard '{0}' already exists")]
    AlreadyExists(String),
    
    #[error("Cannot modify protected shard: {0}")]
    Protected(String),
    
    #[error("Filesystem error at {path}: {source}")]
    Filesystem { 
        path: PathBuf, 
        source: io::Error 
    },
    
    #[error("Manifest error: {0}")]
    ManifestError(String),
    
    #[error("Backup error for shard '{name}': {source}")]
    BackupError { 
        name: String, 
        source: Box<dyn StdError + Send + Sync> 
    },
    
    #[error("Homebrew error: {0}")]
    BrewError(String),
    
    #[error("Package error: {0}")]
    PackageError(String),
    
    #[error("Application error: {0}")]
    ApplicationError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("{0}")]
    Other(String),
    
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    
    #[error(transparent)]
    Io(#[from] std::io::Error),
    
    #[error("User interaction error: {0}")]
    Interaction(String),
}

// Implement From for dialoguer::Error
impl From<dialoguer::Error> for ShardError {
    fn from(err: dialoguer::Error) -> Self {
        ShardError::Interaction(err.to_string())
    }
}

/// Centralized Result type for Shard operations
pub type ShardResult<T> = std::result::Result<T, ShardError>;

/// Extension trait for Result to add context to errors
pub trait ResultExt<T, E> {
    /// Add context to an error
    fn with_context<C, F>(self, context: F) -> ShardResult<T>
    where
        F: FnOnce() -> C + Send + Sync,
        C: fmt::Display + Send + Sync + 'static;
}

impl<T, E> ResultExt<T, E> for std::result::Result<T, E>
where
    E: StdError + Send + Sync + 'static,
{
    fn with_context<C, F>(self, context: F) -> ShardResult<T>
    where
        F: FnOnce() -> C + Send + Sync,
        C: fmt::Display + Send + Sync + 'static,
    {
        self.map_err(|err| {
            ShardError::Anyhow(anyhow::Error::new(err).context(context()))
        })
    }
}

//-------------------------------------------------------------------------------
// Logging
//-------------------------------------------------------------------------------

/// Initialize the logging subsystem
pub fn init_logging(verbosity: Option<u8>) {
    // Only initialize once
    INIT_LOGGER.call_once(|| {
        // Determine log level based on verbosity
        let log_level = match verbosity {
            Some(0) => Level::ERROR,
            Some(1) => Level::WARN,
            Some(2) => Level::INFO,
            Some(3) => Level::DEBUG,
            Some(_) => Level::TRACE,
            None => Level::INFO,
        };
        
        // Create a custom filter that sets the default level
        let filter = EnvFilter::from_default_env()
            .add_directive(format!("shard={}", log_level).parse().unwrap());
        
        // Initialize the tracing subscriber with the custom filter
        let subscriber = FmtSubscriber::builder()
            .with_env_filter(filter)
            .with_target(false)
            .with_ansi(true)
            .finish();
        
        // Set the global default subscriber
        if let Err(e) = tracing::subscriber::set_global_default(subscriber) {
            eprintln!("Warning: Could not set global default tracing subscriber: {}", e);
        } else {
            debug!("Logging initialized at level: {}", log_level);
        }
    });
}

/// Log a success message
pub fn log_success(message: &str) {
    info!("{} {}", style("✓").bold().green(), message);
    println!("{} {}", style("✓").bold().green(), message);
}

/// Log a warning message
pub fn log_warning(message: &str) {
    warn!("{} {}", style("!").bold().yellow(), message);
    println!("{} {}", style("!").bold().yellow(), message);
}

/// Log an error message
pub fn log_error(message: &str) {
    error!("{} {}", style("✗").bold().red(), message);
    eprintln!("{} {}", style("✗").bold().red(), message);
}

/// Log a step message
pub fn log_step(message: &str) {
    info!("{} {}", style("→").bold().blue(), message);
    println!("{} {}", style("→").bold().blue(), message);
}

/// Log a debug message 
pub fn log_debug(message: &str) {
    debug!("{}", message);
    // Only output in verbose mode, handled by tracing
}

/// Log a trace message
pub fn log_trace(message: &str) {
    trace!("{}", message);
    // Only output in very verbose mode, handled by tracing
} 