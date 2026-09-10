//! Structured audit logging for session events.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum SessionEvent {
    SessionStarted {
        timestamp: DateTime<Utc>,
        session_id: String,
        command: String,
    },
    CommandEvaluated {
        timestamp: DateTime<Utc>,
        session_id: String,
        command: String,
        policy_action: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        matched_rule: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    CheckpointCreated {
        timestamp: DateTime<Utc>,
        session_id: String,
        checkpoint_id: String,
        description: String,
    },
    SessionEnded {
        timestamp: DateTime<Utc>,
        session_id: String,
        exit_code: Option<i32>,
    },
}

pub struct SessionLogger {
    log_path: PathBuf,
}

impl SessionLogger {
    pub fn new(leash_dir: &Path) -> Self {
        Self {
            log_path: leash_dir.join("log.jsonl"),
        }
    }

    pub fn log_event(&self, _event: &SessionEvent) -> Result<()> {
        anyhow::bail!("SessionLogger not implemented yet")
    }

    pub fn read_events(
        &self,
        _session_id: Option<&str>,
        _tail: Option<usize>,
    ) -> Result<Vec<SessionEvent>> {
        anyhow::bail!("Reading session log not implemented yet")
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }
}
