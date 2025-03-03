pub mod operations;
pub mod processor;

// Re-export common types
pub use operations::PackageType;
pub use processor::{PackageInfo, PackageOperation, PackageProcessResult, PackageProcessor}; 