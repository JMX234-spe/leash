use anyhow::Result;
use clap::Parser;
use leash::cli::{Cli, Commands};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let default_filter = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter)),
        )
        .init();

    match cli.command {
        Commands::Init(_args) => {
            println!("leash init: not implemented yet");
        }
        Commands::Run(args) => {
            println!(
                "leash run: not implemented yet. Command: {:?}",
                args.command
            );
        }
        Commands::Checkpoints(_args) => {
            println!("leash checkpoints: not implemented yet");
        }
        Commands::Rewind(args) => {
            println!(
                "leash rewind: not implemented yet. Checkpoint: {}",
                args.checkpoint_id
            );
        }
        Commands::Log(_args) => {
            println!("leash log: not implemented yet");
        }
    }

    Ok(())
}
