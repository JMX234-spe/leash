//! Policy evaluation engine.

use crate::policy::schema::{PolicyAction, PolicyConfig, PolicyRule};
use anyhow::Result;
use regex::Regex;

/// Compiled rule with regex for fast evaluation.
#[derive(Debug)]
pub struct CompiledRule {
    pub rule: PolicyRule,
    pub regex: Regex,
}

/// Evaluates command strings against declared policies.
#[derive(Debug)]
pub struct PolicyEngine {
    pub config: PolicyConfig,
    pub compiled_rules: Vec<CompiledRule>,
}

/// Result of evaluating a command string against the policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationResult {
    pub action: PolicyAction,
    pub matched_rule: Option<String>,
    pub reason: Option<String>,
}

impl PolicyEngine {
    /// Builds a new PolicyEngine from a PolicyConfig, compiling regex patterns.
    pub fn new(config: PolicyConfig) -> Result<Self> {
        let mut compiled_rules = Vec::new();
        for rule in &config.rules {
            let regex = Regex::new(&rule.pattern)?;
            compiled_rules.push(CompiledRule {
                rule: rule.clone(),
                regex,
            });
        }
        Ok(Self {
            config,
            compiled_rules,
        })
    }

    /// Evaluates a raw command string.
    ///
    /// Evaluates rules in order; the first matching rule determines the action.
    /// If no rules match, falls back to `default_action`.
    pub fn evaluate(&self, command: &str) -> EvaluationResult {
        for compiled in &self.compiled_rules {
            if compiled.regex.is_match(command) {
                return EvaluationResult {
                    action: compiled.rule.action,
                    matched_rule: Some(compiled.rule.name.clone()),
                    reason: compiled.rule.reason.clone(),
                };
            }
        }

        EvaluationResult {
            action: self.config.default_action,
            matched_rule: None,
            reason: None,
        }
    }
}
