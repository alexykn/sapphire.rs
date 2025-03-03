pub mod observability;
pub mod filesystem;

// Re-export commonly used observability items for convenience
pub use observability::{
    ShardError,
    ShardResult,
    ResultExt,
    log_success,
    log_warning,
    log_error,
    log_step,
    log_debug,
    log_trace,
    Logger,
    LogLevel,
};

// Re-export commonly used helper functions
pub use filesystem::{
    ensure_dir_exists,
    ensure_parent_dir_exists,
    file_exists,
    path_exists,
    copy_file,
    rename_path,
    remove_file,
    backup_file,
}; 