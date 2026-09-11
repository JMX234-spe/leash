//! Policy evaluation engine.

use anyhow::Result;
use regex::Regex;

use crate::policy::schema::{PolicyAction, PolicyConfig, PolicyRule};

/// Compiled rule with regex for fast evaluation.
#[derive(Debug)]
pub(crate) struct CompiledRule {
    pub rule: PolicyRule,
    pub regex: Regex,
}

/// Evaluates command strings against declared policies.
#[derive(Debug)]
pub struct PolicyEngine {
    pub config: PolicyConfig,
    pub(crate) compiled_rules: Vec<CompiledRule>,
}

/// Result of evaluating a command string against the policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationResult {
    pub action: PolicyAction,
    pub matched_rule: Option<String>,
    pub reason: Option<String>,
}

/// Normalizes a command string before policy evaluation:
/// - Strips leading and trailing whitespace
/// - Collapses multiple spaces, tabs, and newlines into a single space
/// - Normalizes simple flag permutations (e.g. `-r -f`, `-f -r`, `--recursive --force` -> `-rf`)
pub fn normalize_command(command: &str) -> String {
    let trimmed = command.trim();
    let mut collapsed = String::with_capacity(trimmed.len());
    let mut in_whitespace = false;

    for ch in trimmed.chars() {
        if ch.is_whitespace() {
            if !in_whitespace {
                collapsed.push(' ');
                in_whitespace = true;
            }
        } else {
            collapsed.push(ch);
            in_whitespace = false;
        }
    }

    collapsed
        .replace("-r -f", "-rf")
        .replace("-f -r", "-rf")
        .replace("-R -f", "-rf")
        .replace("-f -R", "-rf")
        .replace("--recursive -f", "-rf")
        .replace("-f --recursive", "-rf")
        .replace("-r --force", "-rf")
        .replace("--force -r", "-rf")
        .replace("--recursive --force", "-rf")
        .replace("--force --recursive", "-rf")
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

    /// Evaluates a raw command string after normalization.
    ///
    /// Evaluates rules in order; the first matching rule determines the action.
    /// If no rules match, falls back to `default_action`.
    pub fn evaluate(&self, command: &str) -> EvaluationResult {
        let normalized = normalize_command(command);
        for compiled in &self.compiled_rules {
            if compiled.regex.is_match(&normalized) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_command_collapses_whitespace_and_normalizes_destructive_flags() {
        assert_eq!(normalize_command("  rm   -rf    /  "), "rm -rf /");
        assert_eq!(normalize_command("rm\t-r\t-f\t/"), "rm -rf /");
        assert_eq!(normalize_command("rm -f -r /"), "rm -rf /");
        assert_eq!(normalize_command("rm --force --recursive /"), "rm -rf /");
        assert_eq!(
            normalize_command("git   push   --force"),
            "git push --force"
        );
    }
}
