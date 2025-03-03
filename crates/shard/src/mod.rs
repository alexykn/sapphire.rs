// This file provides backward compatibility with the old module structure
// Legacy re-exports - these will be deprecated in a future version

pub mod apply { pub use crate::shard::apply::*; }
pub mod diff { pub use crate::shard::diff::*; }
pub mod init { pub use crate::shard::init::*; }
pub mod manifest { pub use crate::core::manifest::*; }
pub mod brew_client { pub use crate::brew::client::*; }
pub mod search { pub use crate::brew::search::*; }
pub mod package { pub use crate::package::operations::*; }
pub mod migrate { pub use crate::shard::migrate::*; }
pub mod manage { pub use crate::shard::manager::*; }
pub mod package_processor { pub use crate::package::processor::*; }