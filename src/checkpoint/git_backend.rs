//! Git backend for creating, listing, and restoring checkpoints on `refs/leash/checkpoints`.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a recorded checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub description: String,
}

pub struct GitCheckpointBackend;

impl GitCheckpointBackend {
    pub fn new() -> Self {
        Self
    }

    pub fn create_checkpoint(&self, _session_id: &str, _description: &str) -> Result<Checkpoint> {
        anyhow::bail!("Checkpoint creation not implemented yet")
    }

    pub fn list_checkpoints(&self, _session_id: Option<&str>) -> Result<Vec<Checkpoint>> {
        anyhow::bail!("Listing checkpoints not implemented yet")
    }

    pub fn restore_checkpoint(&self, _checkpoint_id: &str) -> Result<()> {
        anyhow::bail!("Checkpoint restore not implemented yet")
    }
}

impl Default for GitCheckpointBackend {
    fn default() -> Self {
        Self::new()
    }
}
