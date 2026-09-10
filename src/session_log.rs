//! Structured audit logging for session events.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
    },
    RewindExecuted {
        timestamp: DateTime<Utc>,
        #[serde(skip_serializing_if = "Option::is_none")]
        session_id: Option<String>,
        checkpoint_id: String,
        description: String,
    },
}

impl SessionEvent {
    pub fn session_id(&self) -> Option<&str> {
        match self {
            Self::SessionStarted { session_id, .. } => Some(session_id.as_str()),
            Self::CommandEvaluated { session_id, .. } => Some(session_id.as_str()),
            Self::CheckpointCreated { session_id, .. } => Some(session_id.as_str()),
            Self::SessionEnded { session_id, .. } => Some(session_id.as_str()),
            Self::RewindExecuted { session_id, .. } => session_id.as_deref(),
        }
    }

    pub fn timestamp(&self) -> &DateTime<Utc> {
        match self {
            Self::SessionStarted { timestamp, .. } => timestamp,
            Self::CommandEvaluated { timestamp, .. } => timestamp,
            Self::CheckpointCreated { timestamp, .. } => timestamp,
            Self::SessionEnded { timestamp, .. } => timestamp,
            Self::RewindExecuted { timestamp, .. } => timestamp,
        }
    }

    pub fn event_type_name(&self) -> &'static str {
        match self {
            Self::SessionStarted { .. } => "START",
            Self::CommandEvaluated { .. } => "POLICY",
            Self::CheckpointCreated { .. } => "CHECKPOINT",
            Self::SessionEnded { .. } => "END",
            Self::RewindExecuted { .. } => "REWIND",
        }
    }
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

    pub fn log_event(&self, event: &SessionEvent) -> Result<()> {
        if let Some(parent) = self.log_path.parent() {
            if !parent.exists() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .with_context(|| format!("Failed to open log file at {}", self.log_path.display()))?;

        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, event)
            .with_context(|| "Failed to serialize session event to JSON")?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        Ok(())
    }

    pub fn read_events(
        &self,
        session_id: Option<&str>,
        tail: Option<usize>,
    ) -> Result<Vec<SessionEvent>> {
        if !self.log_path.exists() {
            return Ok(Vec::new());
        }

        let file = std::fs::File::open(&self.log_path)
            .with_context(|| format!("Failed to open log file at {}", self.log_path.display()))?;

        let reader = BufReader::new(file);
        let mut events = Vec::new();

        for line_res in reader.lines() {
            let line = line_res?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let event: SessionEvent = match serde_json::from_str(trimmed) {
                Ok(ev) => ev,
                Err(e) => {
                    tracing::warn!(
                        "Skipping corrupted log line in {}: {}",
                        self.log_path.display(),
                        e
                    );
                    continue;
                }
            };

            if let Some(target_session) = session_id {
                if event.session_id() != Some(target_session) {
                    continue;
                }
            }

            events.push(event);
        }

        if let Some(n) = tail {
            if events.len() > n {
                events = events.split_off(events.len() - n);
            }
        }

        Ok(events)
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_log_and_read_events_lifecycle() {
        let temp = tempdir().unwrap();
        let logger = SessionLogger::new(temp.path());

        // Verify missing log file returns empty vector
        let empty = logger.read_events(None, None).unwrap();
        assert!(empty.is_empty());

        let t1 = Utc::now();
        let event1 = SessionEvent::SessionStarted {
            timestamp: t1,
            session_id: "sess-01".to_string(),
            command: "echo test".to_string(),
        };
        let event2 = SessionEvent::CommandEvaluated {
            timestamp: t1,
            session_id: "sess-01".to_string(),
            command: "echo test".to_string(),
            policy_action: "allow".to_string(),
            matched_rule: None,
            reason: None,
        };
        let event3 = SessionEvent::CheckpointCreated {
            timestamp: t1,
            session_id: "sess-01".to_string(),
            checkpoint_id: "1234567".to_string(),
            description: "before: echo test".to_string(),
        };
        let event4 = SessionEvent::SessionEnded {
            timestamp: t1,
            session_id: "sess-01".to_string(),
            exit_code: Some(0),
            duration_ms: Some(15),
        };

        logger.log_event(&event1).unwrap();
        logger.log_event(&event2).unwrap();
        logger.log_event(&event3).unwrap();
        logger.log_event(&event4).unwrap();

        let all = logger.read_events(None, None).unwrap();
        assert_eq!(all.len(), 4);
        assert_eq!(all[0], event1);
        assert_eq!(all[1], event2);
        assert_eq!(all[2], event3);
        assert_eq!(all[3], event4);
    }

    #[test]
    fn test_filter_by_session_and_tail() {
        let temp = tempdir().unwrap();
        let logger = SessionLogger::new(temp.path());

        let t = Utc::now();
        let e1 = SessionEvent::SessionStarted {
            timestamp: t,
            session_id: "sess-A".to_string(),
            command: "cmd A1".to_string(),
        };
        let e2 = SessionEvent::SessionStarted {
            timestamp: t,
            session_id: "sess-B".to_string(),
            command: "cmd B1".to_string(),
        };
        let e3 = SessionEvent::SessionStarted {
            timestamp: t,
            session_id: "sess-A".to_string(),
            command: "cmd A2".to_string(),
        };

        logger.log_event(&e1).unwrap();
        logger.log_event(&e2).unwrap();
        logger.log_event(&e3).unwrap();

        // Filter by session A
        let filtered_a = logger.read_events(Some("sess-A"), None).unwrap();
        assert_eq!(filtered_a.len(), 2);
        assert_eq!(filtered_a[0], e1);
        assert_eq!(filtered_a[1], e3);

        // Filter by session B
        let filtered_b = logger.read_events(Some("sess-B"), None).unwrap();
        assert_eq!(filtered_b.len(), 1);
        assert_eq!(filtered_b[0], e2);

        // Tail of 2 events
        let tail_2 = logger.read_events(None, Some(2)).unwrap();
        assert_eq!(tail_2.len(), 2);
        assert_eq!(tail_2[0], e2);
        assert_eq!(tail_2[1], e3);

        // Tail of 1 event with session filter
        let tail_a = logger.read_events(Some("sess-A"), Some(1)).unwrap();
        assert_eq!(tail_a.len(), 1);
        assert_eq!(tail_a[0], e3);
    }
}
