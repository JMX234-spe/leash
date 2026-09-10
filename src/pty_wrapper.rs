//! PTY process execution wrapper.
//!
//! Spawns child commands within a pseudo-terminal and captures I/O.

use anyhow::Result;

/// Spawns a command wrapped in a pseudo-terminal.
pub async fn run_pty(_command: &[String]) -> Result<()> {
    anyhow::bail!("PTY wrapper not implemented yet")
}
