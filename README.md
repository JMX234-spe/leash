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
leash init [--force]
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

## Policy Configuration (`.leash/policy.yaml`)

Policies define declarative regex rules evaluated in sequential order:

```yaml
version: 1
rules:
  - name: "block-destructive-rm"
    match: "rm\\s+-rf\\s+(/|~|\\.\\.)"
    action: deny
    reason: "Destructive command targeting sensitive directory"

  - name: "confirm-force-push"
    match: "git\\s+push\\s+.*--force"
    action: ask
    reason: "Force push may overwrite remote history"

  - name: "block-unlisted-network"
    match: "curl\\s+http"
    action: ask
    reason: "Network request is not in whitelist"

default_action: allow # Fallback action: allow | ask | deny
```

### Policy Actions
- `deny`: Blocks execution immediately and exits with code 126.
- `ask`: Prompts the user interactively `[y/N]`. Aborts with code 1 if declined.
- `allow`: Allows execution to proceed without prompting.

## Scope & Limitations (MVP v0.1)

> [!NOTE]
> **Top-Level Command Interception Limitation:**
> In Leash MVP v0.1, command policy evaluation is applied strictly to the top-level command and arguments supplied to `leash run -- <command> [args...]`.
> Subcommands, shell scripts, or child processes spawned internally by the wrapped AI agent are not intercepted at the OS kernel/syscall level in this release. Deep interception of internal agent subprocesses is planned for v0.2.

## Supported Platforms

Official target platforms are **Linux** and **macOS**. Automated CI runs across Ubuntu and macOS on every commit and pull request.

## License

MIT License. See [LICENSE](LICENSE) for details.
