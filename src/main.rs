use anyhow::Result;
use clap::Parser;
use leash::cli::{Cli, Commands};
use leash::config;
use leash::policy::{PolicyAction, PolicyEngine};
use std::io::{IsTerminal, Write};
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
        Commands::Init(args) => match config::init_leash_dir(args.force) {
            Ok(path) => {
                println!("Initialized Leash configuration at {}", path.display());
            }
            Err(e) => {
                eprintln!("Error initializing Leash: {}", e);
                std::process::exit(1);
            }
        },
        Commands::Run(args) => {
            let full_command = args.command.join(" ");

            // Load and evaluate policy
            let policy_config = config::load_active_policy()?;
            let engine = PolicyEngine::new(policy_config)?;
            let eval = engine.evaluate(&full_command);

            match eval.action {
                PolicyAction::Deny => {
                    eprintln!(
                        "[LEASH BLOCKED] Command blocked by policy rule '{}'",
                        eval.matched_rule.as_deref().unwrap_or("<unnamed>")
                    );
                    if let Some(reason) = &eval.reason {
                        eprintln!("Reason: {}", reason);
                    }
                    eprintln!("Command: {}", full_command);
                    std::process::exit(126);
                }
                PolicyAction::Ask => {
                    eprintln!(
                        "[LEASH PROMPT] Command requires confirmation by policy rule '{}'",
                        eval.matched_rule.as_deref().unwrap_or("<unnamed>")
                    );
                    if let Some(reason) = &eval.reason {
                        eprintln!("Reason: {}", reason);
                    }
                    eprintln!("Command: {}", full_command);

                    // Check if stdin is a terminal (TTY).
                    // In non-interactive environments (CI, piped stdin without TTY),
                    // treat 'ask' as 'deny' to avoid hanging indefinitely.
                    if !std::io::stdin().is_terminal() {
                        eprintln!(
                            "[LEASH BLOCKED] Non-interactive session detected; treating 'ask' policy as deny."
                        );
                        std::process::exit(126);
                    }

                    eprint!("Do you want to proceed? [y/N]: ");
                    std::io::stderr().flush()?;

                    let mut response = String::new();
                    let bytes_read = std::io::stdin().read_line(&mut response)?;
                    if bytes_read == 0 {
                        eprintln!("[LEASH ABORTED] EOF encountered on input; aborting.");
                        std::process::exit(1);
                    }
                    let trimmed = response.trim().to_lowercase();
                    if trimmed != "y" && trimmed != "yes" {
                        eprintln!("[LEASH ABORTED] Command execution cancelled by user.");
                        std::process::exit(1);
                    }
                }
                PolicyAction::Allow => {}
            }

            let result = leash::pty_wrapper::run_pty(&args.command)?;
            std::process::exit(result.exit_code);
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
