pub mod operations;
pub mod processor;

// Re-export common types
pub use operations::PackageTypeWrapper;
pub use processor::{PackageInfo, PackageOperation, PackageProcessResult, PackageProcessor}; 