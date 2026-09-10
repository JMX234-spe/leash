use leash::policy::{PolicyAction, PolicyConfig, PolicyEngine, PolicyRule};

#[test]
fn test_engine_empty_config_default_action() {
    let config = PolicyConfig::default();
    let engine = PolicyEngine::new(config).expect("Failed to initialize engine");

    let result = engine.evaluate("echo hello");
    assert_eq!(result.action, PolicyAction::Allow);
    assert_eq!(result.matched_rule, None);
}

#[test]
fn test_engine_rule_matching() {
    let config = PolicyConfig {
        version: 1,
        rules: vec![PolicyRule {
            name: "block-rm".to_string(),
            pattern: r"rm\s+-rf\s+/".to_string(),
            action: PolicyAction::Deny,
            reason: Some("Dangerous root deletion".to_string()),
        }],
        default_action: PolicyAction::Allow,
    };

    let engine = PolicyEngine::new(config).expect("Failed to initialize engine");

    let res_deny = engine.evaluate("rm -rf /");
    assert_eq!(res_deny.action, PolicyAction::Deny);
    assert_eq!(res_deny.matched_rule.as_deref(), Some("block-rm"));

    let res_allow = engine.evaluate("ls -la");
    assert_eq!(res_allow.action, PolicyAction::Allow);
    assert_eq!(res_allow.matched_rule, None);
}
