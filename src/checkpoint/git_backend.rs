//! Git backend for creating, listing, and restoring checkpoints on `refs/leash/checkpoints`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

pub const CHECKPOINT_REF: &str = "refs/leash/checkpoints";

/// Represents a recorded checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Checkpoint {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub description: String,
}

pub struct GitCheckpointBackend {
    repo_root: PathBuf,
}

impl GitCheckpointBackend {
    /// Opens the git repository containing or parent to `path`.
    pub fn discover(path: &Path) -> Result<Self> {
        let repo = git2::Repository::discover(path)
            .with_context(|| format!("No git repository found at or above {}", path.display()))?;
        let repo_root = repo
            .workdir()
            .context("Repository is bare (no working directory)")?
            .to_path_buf();
        Ok(Self { repo_root })
    }

    /// Discovers the git repository from current working directory.
    pub fn open_current() -> Result<Self> {
        let current_dir = std::env::current_dir().context("Failed to get current directory")?;
        Self::discover(&current_dir)
    }

    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    /// Creates a checkpoint capturing the entire current working tree.
    ///
    /// The checkpoint is saved as a commit in the hidden reference `refs/leash/checkpoints`.
    /// The user's active branch and HEAD are completely untouched.
    /// The `.leash/` directory is explicitly excluded from the snapshot.
    pub fn create_checkpoint(&self, session_id: &str, description: &str) -> Result<Checkpoint> {
        let repo = git2::Repository::open(&self.repo_root)
            .context("Failed to open git repository for checkpoint")?;

        // Prepare in-memory index covering all files in workdir
        let mut index = repo.index().context("Failed to get repository index")?;

        // Update any tracked files that were modified or removed
        let _ = index.update_all(["*"].iter(), None);
        // Add all files including new untracked files (respecting .gitignore)
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .context("Failed to add working directory files to index")?;

        // Explicitly exclude the .leash/ directory from checkpoint snapshot
        let to_remove: Vec<PathBuf> = index
            .iter()
            .filter_map(|entry| {
                let path_str = std::str::from_utf8(&entry.path).ok()?;
                if path_str.starts_with(".leash/")
                    || path_str.starts_with(".leash\\")
                    || path_str == ".leash"
                {
                    Some(PathBuf::from(path_str))
                } else {
                    None
                }
            })
            .collect();

        for path in to_remove {
            let _ = index.remove_path(&path);
        }

        let tree_id = index
            .write_tree_to(&repo)
            .context("Failed to write checkpoint tree object")?;
        let tree = repo
            .find_tree(tree_id)
            .context("Failed to locate written tree")?;

        let timestamp = Utc::now();
        let commit_message = format!(
            "leash: {}\n\nsession_id: {}\ntimestamp: {}\ndescription: {}\n",
            description,
            session_id,
            timestamp.to_rfc3339(),
            description
        );

        let signature = repo
            .signature()
            .unwrap_or_else(|_| git2::Signature::now("Leash", "leash@local").unwrap());

        // Check if refs/leash/checkpoints already exists
        let parent_commit = match repo.find_reference(CHECKPOINT_REF) {
            Ok(reference) => reference.peel_to_commit().ok(),
            Err(_) => None,
        };

        let parents: Vec<&git2::Commit> = match &parent_commit {
            Some(parent) => vec![parent],
            None => Vec::new(),
        };

        let commit_id = repo
            .commit(
                Some(CHECKPOINT_REF),
                &signature,
                &signature,
                &commit_message,
                &tree,
                &parents,
            )
            .context("Failed to create checkpoint commit")?;

        let short_id = format!("{:.7}", commit_id);

        Ok(Checkpoint {
            id: short_id,
            timestamp,
            session_id: session_id.to_string(),
            description: description.to_string(),
        })
    }

    /// Lists recorded checkpoints on `refs/leash/checkpoints`, optionally filtered by `session_id`.
    pub fn list_checkpoints(&self, session_id_filter: Option<&str>) -> Result<Vec<Checkpoint>> {
        let repo =
            git2::Repository::open(&self.repo_root).context("Failed to open git repository")?;

        let reference = match repo.find_reference(CHECKPOINT_REF) {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()), // No checkpoints yet
        };

        let head_commit = reference
            .peel_to_commit()
            .context("Failed to peel checkpoint ref to commit")?;

        let mut revwalk = repo.revwalk().context("Failed to create revwalk")?;
        revwalk
            .push(head_commit.id())
            .context("Failed to push checkpoint commit to revwalk")?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let mut checkpoints = Vec::new();

        for commit_oid_result in revwalk {
            let oid = commit_oid_result.context("Failed to get commit OID in revwalk")?;
            let commit = repo.find_commit(oid).context("Failed to find commit")?;
            let short_id = format!("{:.7}", oid);

            let msg = commit.message().unwrap_or_default();
            let parsed = parse_commit_message(&short_id, &commit, msg);

            if let Some(filter) = session_id_filter {
                if parsed.session_id != filter {
                    continue;
                }
            }

            checkpoints.push(parsed);
        }

        Ok(checkpoints)
    }

    /// Restores the working tree and index to the exact state of `checkpoint_id`.
    ///
    /// The user's current branch reference and HEAD remain unchanged.
    /// The `.leash/` directory is preserved across rewinds.
    pub fn restore_checkpoint(&self, checkpoint_id: &str) -> Result<Checkpoint> {
        let repo =
            git2::Repository::open(&self.repo_root).context("Failed to open git repository")?;

        let checkpoint_git_obj = repo
            .revparse_single(checkpoint_id)
            .with_context(|| format!("Checkpoint '{}' not found", checkpoint_id))?;

        let commit = checkpoint_git_obj
            .peel_to_commit()
            .with_context(|| format!("Object '{}' is not a commit", checkpoint_id))?;

        let tree = commit
            .tree()
            .context("Failed to get tree from checkpoint commit")?;

        let short_id = format!("{:.7}", commit.id());
        let msg = commit.message().unwrap_or_default();
        let checkpoint_info = parse_commit_message(&short_id, &commit, msg);

        // Pre-rewind safety snapshot ensures uncommitted work is recoverable.
        let pre_rewind_desc = format!("pre-rewind backup: before restoring {}", checkpoint_id);
        let _ = self.create_checkpoint("rewind-backup", &pre_rewind_desc)?;

        // Local .leash.bak backup preserves audit logs and policy across destructive checkout.
        let leash_dir = self.repo_root.join(".leash");
        let backup_dir = self.repo_root.join(".leash.bak");
        let had_leash = leash_dir.exists();

        if had_leash {
            if backup_dir.exists() {
                let _ = std::fs::remove_dir_all(&backup_dir);
            }
            copy_dir_all(&leash_dir, &backup_dir)
                .context("Failed to create local .leash.bak backup before rewind")?;
        }

        // In-memory ignore rule guarantees checkout_tree(remove_untracked=true) ignores .leash.
        let _ = repo.add_ignore_rule(".leash\n.leash/*\n.leash.bak\n.leash.bak/*\n");

        // Perform forced checkout to restore working directory files.
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.force();
        checkout.remove_untracked(true);

        let checkout_res = repo.checkout_tree(tree.as_object(), Some(&mut checkout));
        if let Err(e) = checkout_res {
            if had_leash && !leash_dir.exists() && backup_dir.exists() {
                let _ = copy_dir_all(&backup_dir, &leash_dir);
            }
            return Err(e).context("Failed to checkout checkpoint tree into working directory");
        }

        // Synchronize repository index with the restored tree.
        let mut index = repo.index().context("Failed to open index")?;
        let sync_res = index.read_tree(&tree).and_then(|_| index.write());
        if let Err(e) = sync_res {
            if had_leash && !leash_dir.exists() && backup_dir.exists() {
                let _ = copy_dir_all(&backup_dir, &leash_dir);
            }
            return Err(e).context("Failed to sync index with restored tree");
        }

        // Restore .leash from local backup if affected, then clean up .leash.bak.
        if had_leash {
            if !leash_dir.exists() && backup_dir.exists() {
                copy_dir_all(&backup_dir, &leash_dir)
                    .context("Failed to restore .leash directory from local backup")?;
            }
            if backup_dir.exists() {
                let _ = std::fs::remove_dir_all(&backup_dir);
            }
        }

        Ok(checkpoint_info)
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}

fn parse_commit_message(short_id: &str, commit: &git2::Commit, message: &str) -> Checkpoint {
    let mut session_id = String::new();
    let mut timestamp: Option<DateTime<Utc>> = None;
    let mut description = String::new();

    for line in message.lines() {
        let trimmed = line.trim();
        if let Some(parsed_session) = trimmed.strip_prefix("session_id:") {
            session_id = parsed_session.trim().to_string();
        } else if let Some(parsed_timestamp_str) = trimmed.strip_prefix("timestamp:") {
            if let Ok(dt) = DateTime::parse_from_rfc3339(parsed_timestamp_str.trim()) {
                timestamp = Some(dt.with_timezone(&Utc));
            }
        } else if let Some(parsed_desc) = trimmed.strip_prefix("description:") {
            description = parsed_desc.trim().to_string();
        }
    }

    if session_id.is_empty() {
        session_id = "unknown".to_string();
    }

    let timestamp = timestamp.unwrap_or_else(|| {
        let seconds = commit.time().seconds();
        Utc.timestamp_opt(seconds, 0)
            .single()
            .unwrap_or_else(Utc::now)
    });

    if description.is_empty() {
        description = commit.summary().unwrap_or("checkpoint").to_string();
    }

    Checkpoint {
        id: short_id.to_string(),
        timestamp,
        session_id,
        description,
    }
}
