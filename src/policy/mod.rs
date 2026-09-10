//! Policy engine module for declarative security rule enforcement.
//!
//! Evaluates commands against regex patterns configured in `.leash/policy.yaml`.
//!
//! # Scope Limitation (MVP v0.1)
//! In Leash v0.1, command interception and evaluation is strictly applied to the
//! top-level command invocation passed to `leash run -- <cmd> [args...]`.
//! Internal subcommands executed inside shell scripts or spawned child processes
//! by the wrapped agent are not intercepted at the OS kernel/syscall level in this
//! version. Transparent kernel-level or sub-process interception is planned for v0.2+.

pub mod engine;
pub mod schema;

pub use engine::{EvaluationResult, PolicyEngine};
pub use schema::{PolicyAction, PolicyConfig, PolicyRule};
