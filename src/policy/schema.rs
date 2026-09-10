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

impl PolicyAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            PolicyAction::Allow => "allow",
            PolicyAction::Ask => "ask",
            PolicyAction::Deny => "deny",
        }
    }
}

impl std::fmt::Display for PolicyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
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
