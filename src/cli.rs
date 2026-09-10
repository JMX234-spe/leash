use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "leash",
    author,
    version,
    about = "Transparent security wrapper and rewind engine for AI coding agents",
    long_about = "Leash wraps arbitrary coding agents via PTY, evaluates execution policies, records structured audit logs, and creates git-based checkpoints with rewind capabilities."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Verbose output for debugging
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize .leash/ directory in current repository with an example policy
    Init(InitArgs),

    /// Execute a command wrapped in a PTY with policy enforcement and checkpointing
    Run(RunArgs),

    /// List available checkpoints
    Checkpoints(CheckpointsArgs),

    /// Revert the working tree to a specific checkpoint
    Rewind(RewindArgs),

    /// View structured session log events
    Log(LogArgs),
}

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Force overwrite of existing policy file if present
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct RunArgs {
    /// Command and arguments to execute wrapped by Leash
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
    pub command: Vec<String>,
}

#[derive(Debug, Args)]
pub struct CheckpointsArgs {
    /// Filter checkpoints by session ID
    #[arg(short, long)]
    pub session: Option<String>,
}

#[derive(Debug, Args)]
pub struct RewindArgs {
    /// ID or hash prefix of the checkpoint to restore
    pub checkpoint_id: String,

    /// Proceed without asking for confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

#[derive(Debug, Args)]
pub struct LogArgs {
    /// Filter logs by session ID
    #[arg(short, long)]
    pub session: Option<String>,

    /// Show only the last N events
    #[arg(short = 'n', long)]
    pub tail: Option<usize>,

    /// Output raw JSONL instead of formatted human-readable table
    #[arg(long)]
    pub json: bool,
}
