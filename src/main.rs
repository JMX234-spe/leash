use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::time::{Instant, SystemTime};

use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use leash::checkpoint::GitCheckpointBackend;
use leash::cli::{CheckpointsArgs, Cli, Commands, InitArgs, LogArgs, RewindArgs, RunArgs};
use leash::config;
use leash::policy::{PolicyAction, PolicyEngine};
use leash::session_log::{SessionEvent, SessionLogger};

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
        Commands::Init(args) => handle_init(args),
        Commands::Run(args) => handle_run(args),
        Commands::Checkpoints(args) => handle_checkpoints(args),
        Commands::Rewind(args) => handle_rewind(args),
        Commands::Log(args) => handle_log(args),
    }
}

fn handle_init(args: InitArgs) -> Result<()> {
    match config::init_leash_dir(args.force) {
        Ok(path) => {
            println!("Initialized Leash configuration at {}", path.display());
            Ok(())
        }
        Err(e) => {
            eprintln!("[LEASH ERROR] Failed to initialize Leash: {}", e);
            std::process::exit(1);
        }
    }
}

fn handle_run(args: RunArgs) -> Result<()> {
    let session_id = generate_session_id();
    let full_command = args.command.join(" ");
    let start_instant = Instant::now();

    let leash_dir = config::find_leash_dir().unwrap_or_else(|_| PathBuf::from(".leash"));
    let logger = SessionLogger::new(&leash_dir);

    let _ = logger.log_event(&SessionEvent::SessionStarted {
        timestamp: Utc::now(),
        session_id: session_id.clone(),
        command: full_command.clone(),
    });

    let policy_config = match config::load_active_policy() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("[LEASH ERROR] Failed to load policy: {}", e);
            let _ = logger.log_event(&SessionEvent::SessionEnded {
                timestamp: Utc::now(),
                session_id: session_id.clone(),
                exit_code: Some(1),
                duration_ms: Some(start_instant.elapsed().as_millis() as u64),
            });
            std::process::exit(1);
        }
    };

    let engine = match PolicyEngine::new(policy_config) {
        Ok(eng) => eng,
        Err(e) => {
            eprintln!("[LEASH ERROR] Invalid policy configuration: {}", e);
            let _ = logger.log_event(&SessionEvent::SessionEnded {
                timestamp: Utc::now(),
                session_id: session_id.clone(),
                exit_code: Some(1),
                duration_ms: Some(start_instant.elapsed().as_millis() as u64),
            });
            std::process::exit(1);
        }
    };

    let eval = engine.evaluate(&full_command);

    let _ = logger.log_event(&SessionEvent::CommandEvaluated {
        timestamp: Utc::now(),
        session_id: session_id.clone(),
        command: full_command.clone(),
        policy_action: eval.action.to_string(),
        matched_rule: eval.matched_rule.clone(),
        reason: eval.reason.clone(),
    });

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
            let _ = logger.log_event(&SessionEvent::SessionEnded {
                timestamp: Utc::now(),
                session_id: session_id.clone(),
                exit_code: Some(126),
                duration_ms: Some(start_instant.elapsed().as_millis() as u64),
            });
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

            if !std::io::stdin().is_terminal() {
                eprintln!(
                    "[LEASH BLOCKED] Non-interactive session detected; treating 'ask' policy as deny."
                );
                let _ = logger.log_event(&SessionEvent::SessionEnded {
                    timestamp: Utc::now(),
                    session_id: session_id.clone(),
                    exit_code: Some(126),
                    duration_ms: Some(start_instant.elapsed().as_millis() as u64),
                });
                std::process::exit(126);
            }

            eprint!("Do you want to proceed? [y/N]: ");
            std::io::stderr().flush()?;

            let mut response = String::new();
            let bytes_read = std::io::stdin().read_line(&mut response)?;
            if bytes_read == 0 {
                eprintln!("[LEASH ABORTED] EOF encountered on input; aborting.");
                let _ = logger.log_event(&SessionEvent::SessionEnded {
                    timestamp: Utc::now(),
                    session_id: session_id.clone(),
                    exit_code: Some(1),
                    duration_ms: Some(start_instant.elapsed().as_millis() as u64),
                });
                std::process::exit(1);
            }
            let trimmed = response.trim().to_lowercase();
            if trimmed != "y" && trimmed != "yes" {
                eprintln!("[LEASH ABORTED] Command execution cancelled by user.");
                let _ = logger.log_event(&SessionEvent::SessionEnded {
                    timestamp: Utc::now(),
                    session_id: session_id.clone(),
                    exit_code: Some(1),
                    duration_ms: Some(start_instant.elapsed().as_millis() as u64),
                });
                std::process::exit(1);
            }
        }
        PolicyAction::Allow => {}
    }

    let backend = match GitCheckpointBackend::open_current() {
        Ok(b) => b,
        Err(_) => {
            eprintln!("[LEASH ERROR] Leash requires a git repository. Run 'git init' first.");
            let _ = logger.log_event(&SessionEvent::SessionEnded {
                timestamp: Utc::now(),
                session_id: session_id.clone(),
                exit_code: Some(1),
                duration_ms: Some(start_instant.elapsed().as_millis() as u64),
            });
            std::process::exit(1);
        }
    };

    let checkpoint_desc = format!("before: {}", full_command);
    match backend.create_checkpoint(&session_id, &checkpoint_desc) {
        Ok(checkpoint) => {
            eprintln!("[LEASH] Created pre-execution checkpoint {}", checkpoint.id);
            let _ = logger.log_event(&SessionEvent::CheckpointCreated {
                timestamp: Utc::now(),
                session_id: session_id.clone(),
                checkpoint_id: checkpoint.id,
                description: checkpoint_desc,
            });
        }
        Err(e) => {
            tracing::warn!("Could not create automatic checkpoint: {}", e);
        }
    }

    let pty_result = match leash::pty_wrapper::run_pty(&args.command) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("[LEASH ERROR] Command execution failed: {}", e);
            let _ = logger.log_event(&SessionEvent::SessionEnded {
                timestamp: Utc::now(),
                session_id: session_id.clone(),
                exit_code: Some(1),
                duration_ms: Some(start_instant.elapsed().as_millis() as u64),
            });
            std::process::exit(1);
        }
    };

    let _ = logger.log_event(&SessionEvent::SessionEnded {
        timestamp: Utc::now(),
        session_id: session_id.clone(),
        exit_code: Some(pty_result.exit_code),
        duration_ms: Some(start_instant.elapsed().as_millis() as u64),
    });

    std::process::exit(pty_result.exit_code);
}

fn handle_checkpoints(args: CheckpointsArgs) -> Result<()> {
    let backend = match GitCheckpointBackend::open_current() {
        Ok(b) => b,
        Err(_) => {
            eprintln!("[LEASH ERROR] Leash requires a git repository. Run 'git init' first.");
            std::process::exit(1);
        }
    };

    let checkpoints = match backend.list_checkpoints(args.session.as_deref()) {
        Ok(cp) => cp,
        Err(e) => {
            eprintln!("[LEASH ERROR] Failed to list checkpoints: {}", e);
            std::process::exit(1);
        }
    };

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
        for checkpoint in checkpoints {
            println!(
                "{:<10} {:<10} {:<25} {}",
                checkpoint.id,
                checkpoint.session_id,
                checkpoint.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                checkpoint.description
            );
        }
    }
    Ok(())
}

fn handle_rewind(args: RewindArgs) -> Result<()> {
    let backend = match GitCheckpointBackend::open_current() {
        Ok(b) => b,
        Err(_) => {
            eprintln!("[LEASH ERROR] Leash requires a git repository. Run 'git init' first.");
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
        Ok(restored_checkpoint) => {
            println!(
                "[LEASH] Successfully rewound working directory to checkpoint {} ({})",
                restored_checkpoint.id, restored_checkpoint.description
            );
            if let Ok(leash_dir) = config::find_leash_dir() {
                let logger = SessionLogger::new(&leash_dir);
                let _ = logger.log_event(&SessionEvent::RewindExecuted {
                    timestamp: Utc::now(),
                    session_id: Some(restored_checkpoint.session_id.clone()),
                    checkpoint_id: restored_checkpoint.id.clone(),
                    description: restored_checkpoint.description.clone(),
                });
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("[LEASH ERROR] Failed to restore checkpoint: {}", e);
            std::process::exit(1);
        }
    }
}

fn handle_log(args: LogArgs) -> Result<()> {
    let leash_dir = match config::find_leash_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("[LEASH ERROR] Failed to determine Leash directory: {}", e);
            std::process::exit(1);
        }
    };

    let logger = SessionLogger::new(&leash_dir);
    let events = match logger.read_events(args.session.as_deref(), args.tail) {
        Ok(ev) => ev,
        Err(e) => {
            eprintln!("[LEASH ERROR] Failed to read session log: {}", e);
            std::process::exit(1);
        }
    };

    if events.is_empty() {
        println!("No session log entries found.");
        return Ok(());
    }

    if args.json {
        for event in events {
            println!("{}", serde_json::to_string(&event)?);
        }
    } else {
        println!(
            "{:<20} {:<10} {:<12} DETAILS",
            "TIMESTAMP (UTC)", "SESSION", "EVENT"
        );
        println!(
            "{:<20} {:<10} {:<12} ----------------------------------------",
            "-------------------", "---------", "-----------"
        );
        for event in events {
            let timestamp_str = event.timestamp().format("%Y-%m-%d %H:%M:%S").to_string();
            let session_label = event.session_id().unwrap_or("-");
            let event_tag = event.event_type_name();
            let details = match &event {
                SessionEvent::SessionStarted { command, .. } => {
                    format!("command: {}", command)
                }
                SessionEvent::CommandEvaluated {
                    policy_action,
                    matched_rule,
                    reason,
                    ..
                } => {
                    let rule_name = matched_rule.as_deref().unwrap_or("<default>");
                    if let Some(r) = reason {
                        format!("{} (rule: '{}', reason: '{}')", policy_action, rule_name, r)
                    } else {
                        format!("{} (rule: '{}')", policy_action, rule_name)
                    }
                }
                SessionEvent::CheckpointCreated {
                    checkpoint_id,
                    description,
                    ..
                } => format!("{} ({})", checkpoint_id, description),
                SessionEvent::SessionEnded {
                    exit_code,
                    duration_ms,
                    ..
                } => {
                    let code_str = exit_code
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "none".to_string());
                    let dur_str = duration_ms
                        .map(|d| format!(" ({}ms)", d))
                        .unwrap_or_default();
                    format!("exit code: {}{}", code_str, dur_str)
                }
                SessionEvent::RewindExecuted {
                    checkpoint_id,
                    description,
                    ..
                } => format!("restored to {} ({})", checkpoint_id, description),
            };

            println!(
                "{:<20} {:<10} {:<12} {}",
                timestamp_str, session_label, event_tag, details
            );
        }
    }

    Ok(())
}
