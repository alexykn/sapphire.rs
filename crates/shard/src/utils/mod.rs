pub mod observability;
pub mod helpers;

// Re-export commonly used observability items for convenience
pub use observability::{
    ShardError,
    ShardResult,
    ResultExt,
    init_logging,
    log_success,
    log_warning,
    log_error,
    log_step,
    log_debug,
    log_trace,
};

// Re-export commonly used helper functions
pub use helpers::{
    ensure_dir_exists,
    ensure_parent_dir_exists,
    file_exists,
    path_exists,
    copy_file,
    rename_path,
    remove_file,
    backup_file,
}; 