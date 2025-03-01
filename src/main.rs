mod cli;
mod core;
mod sapphire;
mod shard;
mod fragment;
mod utils;

fn main() -> anyhow::Result<()> {
    cli::run()
}
