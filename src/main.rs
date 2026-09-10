use anyhow::Result;
use clap::Parser;
use leash::checkpoint::GitCheckpointBackend;
use leash::cli::{Cli, Commands};
use leash::config;
use leash::policy::{PolicyAction, PolicyEngine};
use std::io::{IsTerminal, Write};
use std::time::SystemTime;
use tracing_subscriber::EnvFilter;

fn generate_session_id() -> String {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:08x}", duration.as_millis() as u64 & 0xffffffff)
}

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

            // Ensure we are inside a git repository for checkpointing
            let backend = match GitCheckpointBackend::open_current() {
                Ok(b) => b,
                Err(_) => {
                    eprintln!(
                        "[LEASH ERROR] Leash requires a git repository. Run 'git init' first."
                    );
                    std::process::exit(1);
                }
            };

            // Create pre-execution checkpoint
            let session_id = generate_session_id();
            let desc = format!("before: {}", full_command);
            match backend.create_checkpoint(&session_id, &desc) {
                Ok(cp) => {
                    eprintln!("[LEASH] Created pre-execution checkpoint {}", cp.id);
                }
                Err(e) => {
                    tracing::warn!("Could not create automatic checkpoint: {}", e);
                }
            }

            let result = leash::pty_wrapper::run_pty(&args.command)?;
            std::process::exit(result.exit_code);
        }
        Commands::Checkpoints(args) => {
            let backend = match GitCheckpointBackend::open_current() {
                Ok(b) => b,
                Err(_) => {
                    eprintln!(
                        "[LEASH ERROR] Leash requires a git repository. Run 'git init' first."
                    );
                    std::process::exit(1);
                }
            };

            let checkpoints = backend.list_checkpoints(args.session.as_deref())?;
            if checkpoints.is_empty() {
                println!("No checkpoints recorded.");
            } else {
                println!(
                    "{:<10} {:<10} {:<25} DESCRIPTION",
                    "ID", "SESSION", "TIMESTAMP"
                );
                println!(
                    "{:<10} {:<10} {:<25} -----------",
                    "-------", "-------", "-------------------------"
                );
                for cp in checkpoints {
                    println!(
                        "{:<10} {:<10} {:<25} {}",
                        cp.id,
                        cp.session_id,
                        cp.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                        cp.description
                    );
                }
            }
        }
        Commands::Rewind(args) => {
            let backend = match GitCheckpointBackend::open_current() {
                Ok(b) => b,
                Err(_) => {
                    eprintln!(
                        "[LEASH ERROR] Leash requires a git repository. Run 'git init' first."
                    );
                    std::process::exit(1);
                }
            };

            if !args.yes {
                if !std::io::stdin().is_terminal() {
                    eprintln!(
                        "[LEASH ERROR] Cannot prompt for confirmation in non-interactive mode. Use --yes to confirm rewind."
                    );
                    std::process::exit(1);
                }

                eprint!(
                    "Are you sure you want to rewind working directory to checkpoint '{}'?\nAny uncommitted changes will be replaced. [y/N]: ",
                    args.checkpoint_id
                );
                std::io::stderr().flush()?;

                let mut response = String::new();
                let bytes_read = std::io::stdin().read_line(&mut response)?;
                if bytes_read == 0 {
                    eprintln!("[LEASH ABORTED] EOF encountered on input; aborting.");
                    std::process::exit(1);
                }
                let trimmed = response.trim().to_lowercase();
                if trimmed != "y" && trimmed != "yes" {
                    eprintln!("[LEASH ABORTED] Rewind cancelled by user.");
                    std::process::exit(1);
                }
            }

            match backend.restore_checkpoint(&args.checkpoint_id) {
                Ok(cp) => {
                    println!(
                        "[LEASH] Successfully rewound working directory to checkpoint {} ({})",
                        cp.id, cp.description
                    );
                }
                Err(e) => {
                    eprintln!("Error during rewind: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Log(_args) => {
            println!("leash log: not implemented yet");
        }
    }

    Ok(())
}
