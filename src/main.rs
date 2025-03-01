// This is a thin wrapper around the sapphire-cli functionality
fn main() -> anyhow::Result<()> {
    // Use the sapphire-core crate's functionality
    sapphire_core::cli::run()
}
