// This is a thin wrapper around the sapphire-cli functionality
fn main() -> anyhow::Result<()> {
    // Use the sapphire crate's functionality
    sapphire::cli::run()
}
