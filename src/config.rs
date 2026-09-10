//! Configuration and policy file loader.

use crate::policy::schema::PolicyConfig;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

pub const LEASH_DIR: &str = ".leash";
pub const POLICY_FILE: &str = "policy.yaml";

pub const DEFAULT_POLICY_YAML: &str = r#"version: 1
rules:
  - name: "block-destructive-rm"
    match: "rm\\s+-rf\\s+(/|~|\\.\\.)"
    action: deny
    reason: "Comando destructivo detectado sobre una ruta sensible"

  - name: "confirm-force-push"
    match: "git\\s+push\\s+.*--force"
    action: ask
    reason: "Push forzado puede sobrescribir historial remoto"

  - name: "block-unlisted-network"
    match: "curl\\s+http"
    action: ask
    reason: "Llamada de red no está en la whitelist"

default_action: allow   # qué hacer si ningún patrón coincide: allow | ask | deny
"#;

/// Returns the `.leash` directory path relative to the current working directory,
/// or from the root of the containing git repository if it exists there.
pub fn find_leash_dir() -> Result<PathBuf> {
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    if let Ok(repo) = git2::Repository::discover(&current_dir) {
        if let Some(workdir) = repo.workdir() {
            let root_leash = workdir.join(LEASH_DIR);
            if root_leash.exists() {
                return Ok(root_leash);
            }
        }
    }
    let leash_dir = current_dir.join(LEASH_DIR);
    Ok(leash_dir)
}

/// Initializes the `.leash` directory and writes an example `policy.yaml`.
pub fn init_leash_dir(force: bool) -> Result<PathBuf> {
    let leash_dir = find_leash_dir()?;
    if !leash_dir.exists() {
        std::fs::create_dir_all(&leash_dir)
            .with_context(|| format!("Failed to create directory {}", leash_dir.display()))?;
    }

    let policy_path = leash_dir.join(POLICY_FILE);
    if policy_path.exists() && !force {
        bail!(
            "Policy file already exists at {}. Use --force to overwrite.",
            policy_path.display()
        );
    }

    std::fs::write(&policy_path, DEFAULT_POLICY_YAML)
        .with_context(|| format!("Failed to write policy file at {}", policy_path.display()))?;

    Ok(policy_path)
}

/// Loads a policy configuration from an explicit file path.
pub fn load_policy(path: &Path) -> Result<PolicyConfig> {
    if !path.exists() {
        return Ok(PolicyConfig::default());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read policy file at {}", path.display()))?;
    let config: PolicyConfig = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse YAML policy at {}", path.display()))?;
    Ok(config)
}

/// Loads the active policy from `.leash/policy.yaml` if it exists,
/// otherwise falls back to the default policy configuration.
pub fn load_active_policy() -> Result<PolicyConfig> {
    if let Ok(leash_dir) = find_leash_dir() {
        let policy_path = leash_dir.join(POLICY_FILE);
        if policy_path.exists() {
            return load_policy(&policy_path);
        }
    }
    Ok(PolicyConfig::default())
}
