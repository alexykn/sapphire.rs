pub mod client;
pub mod search;

// Re-export common types and functions
pub use client::BrewClient;

// Convenience function to get a brew client
pub fn get_client() -> client::BrewClient {
    client::BrewClient::new()
} 