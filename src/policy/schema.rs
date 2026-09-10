//! Serde schemas for policy configuration files.

use serde::{Deserialize, Serialize};

/// Action taken when a rule matches or as the default action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolicyAction {
    Allow,
    Ask,
    Deny,
}

/// A single declarative security rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub name: String,
    #[serde(rename = "match")]
    pub pattern: String,
    pub action: PolicyAction,
    #[serde(default)]
    pub reason: Option<String>,
}

/// Complete policy file definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub rules: Vec<PolicyRule>,
    #[serde(default = "default_action")]
    pub default_action: PolicyAction,
}

fn default_version() -> u32 {
    1
}

fn default_action() -> PolicyAction {
    PolicyAction::Allow
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            version: 1,
            rules: Vec::new(),
            default_action: PolicyAction::Allow,
        }
    }
}
