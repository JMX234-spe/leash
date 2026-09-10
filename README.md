# Leash 🦮

> Transparent security wrapper and rewind engine for AI coding agents.

Leash wraps the execution of arbitrary AI coding agents (Claude Code, Cursor CLI, Codex CLI, Aider, etc.) via a pseudo-terminal (PTY) without modifying their internals.

## Key Features

- **PTY Process Wrapper:** Runs target commands transparently while capturing I/O.
- **Declarative Policy Engine:** Intercepts and validates shell commands against YAML policies (`allow`, `ask`, `deny`).
- **Git-Based Checkpointing:** Automatically tracks repository states on hidden refs (`refs/leash/checkpoints`) before risky operations.
- **Rewind Capability:** Easily revert working tree changes to prior checkpoints with `leash rewind`.
- **Structured Audit Logging:** 100% local session recording in `.leash/log.jsonl`.
- **Zero Telemetry:** Fully local execution with zero network telemetry.

## CLI Commands

```bash
leash init
    Initialize .leash/ directory and an initial policy.yaml template.

leash run -- <command> [args...]
    Wrap <command> in a PTY session with policy enforcement and checkpointing.

leash checkpoints [--session <id>]
    List recorded checkpoints (id, timestamp, description, session).

leash rewind <checkpoint_id> [--yes]
    Restore the working tree to a recorded checkpoint state.

leash log [--session <id>] [--tail N]
    Display structured session events in human-readable output.
```

## License

MIT License. See [LICENSE](LICENSE) for details.
