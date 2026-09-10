//! Configuration and policy file loader.

use crate::policy::schema::PolicyConfig;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub const LEASH_DIR: &str = ".leash";
pub const POLICY_FILE: &str = "policy.yaml";

pub fn find_leash_dir() -> Result<PathBuf> {
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    let leash_dir = current_dir.join(LEASH_DIR);
    Ok(leash_dir)
}

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
