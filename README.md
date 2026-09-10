# Leash 🦮

[![CI](https://github.com/JMX234-/leash/actions/workflows/ci.yml/badge.svg)](https://github.com/JMX234-/leash/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust: 2021](https://img.shields.io/badge/Rust-2021-orange.svg)](https://www.rust-lang.org)

> **Transparent security wrapper and git-based rewind engine for AI coding agents.**

Leash wraps the execution of arbitrary AI coding agents (`claude`, `cursor`, `aider`, `codex`, shell scripts, etc.) via a pseudo-terminal (PTY) without requiring any modifications to those tools. It provides three core capabilities:

1. **Declarative Security Policy Engine**: Intercepts commands against rules declared in `.leash/policy.yaml` (`allow`, `ask`, `deny`).
2. **Git-Based Checkpoints & Rewind**: Automatically snapshots working tree state to a hidden git ref (`refs/leash/checkpoints`) before execution and allows instantaneous restoration (`leash rewind`).
3. **Structured Audit Trail**: Records 100% local, zero-telemetry JSONL audit events in `.leash/log.jsonl`.

---

## Architecture Overview

```
                        +---------------------------------------------+
                        |                 User Shell                  |
                        +---------------------------------------------+
                                               |
                                        leash run -- <cmd>
                                               v
+-----------------------------------------------------------------------------------------+
| LEASH EXECUTION HARNESS                                                                 |
|                                                                                         |
|  1. Policy Engine             2. Hidden Git Checkpoint            3. PTY Wrapper        |
|  +--------------------+       +---------------------------+       +-------------------+ |
|  | Evaluates command  |       | Tree snapshot committed   |       | Spawns agent in   | |
|  | against regexes in | ----> | to hidden git ref:        | ----> | pseudo-terminal   | |
|  | .leash/policy.yaml |       | refs/leash/checkpoints    |       | (portable-pty)    | |
|  +--------------------+       +---------------------------+       +-------------------+ |
|            |                                |                               |           |
|            v                                v                               v           |
|  +-----------------------------------------------------------------------------------+  |
|  | Structured Audit Logger (.leash/log.jsonl - Append Only, Preserved on Rewind)     |  |
|  +-----------------------------------------------------------------------------------+  |
+-----------------------------------------------------------------------------------------+
                                               |
                                               v
                        +---------------------------------------------+
                        |       Target Agent / Subprocess Output      |
                        |      (Transparent Bidirectional I/O)        |
                        +---------------------------------------------+
```

---

## Installation

### Prerequisites
- **Rust toolchain** (Rust 1.75+ or 2021 edition): [rustup.rs](https://rustup.rs)
- **Git** (for checkpointing backend): [git-scm.com](https://git-scm.com)
- **Supported Operating Systems**: Linux (`x86_64`, `aarch64`) and macOS (`x86_64`, Apple Silicon). Windows is supported for local development.

### Build & Install from Source

```bash
# Clone repository
git clone https://github.com/JMX234-/leash.git
cd leash

# Build release binary
cargo build --release

# Install locally to ~/.cargo/bin
cargo install --path .
```

Verify the installation:
```bash
leash --version
leash --help
```

---

## Quickstart

### 1. Initialize Leash in your Repository
Inside any Git repository, run:
```bash
leash init
```
This creates `.leash/policy.yaml` with recommended default security rules.

### 2. Wrap an AI Agent or Command
Prefix your usual agent invocation with `leash run --`:
```bash
# Wrap Claude Code
leash run -- claude "fix the authentication bug"

# Wrap Aider
leash run -- aider --model sonnet

# Wrap any arbitrary shell command or script
leash run -- bash -c "pytest tests/"
```

Before the command begins:
- The policy engine evaluates the top-level command.
- If allowed, a snapshot is automatically saved to `refs/leash/checkpoints`.
- The agent runs interactively with full PTY color, cursor controls, and streaming I/O.
- The command's exit code is transparently propagated back to your shell.

### 3. Inspect Checkpoints
List all recorded checkpoints:
```bash
leash checkpoints
```
Example output:
```
ID         SESSION    TIMESTAMP                 DESCRIPTION
-------    -------    ------------------------- -----------
f3b9a1c    a1b2c3d4   2026-09-10 18:25:00 UTC   before: claude "fix the authentication bug"
```

### 4. Rewind Risky Changes
If the AI agent made unwanted edits or broke your working tree:
```bash
# Rewind to a checkpoint (interactive confirmation prompt)
leash rewind f3b9a1c

# Or proceed non-interactively in scripts
leash rewind f3b9a1c --yes
```
Leash restores all tracked files and removes newly created untracked garbage while keeping your user branch (`main`/`master`) and commit history intact.

### 5. Audit Session Logs
View the structured audit log:
```bash
# View formatted audit trail
leash log

# Filter by session ID
leash log --session a1b2c3d4

# View only the last N events
leash log -n 5

# Output raw JSONL (ideal for piping into jq)
leash log --json | jq .
```

---

## CLI Reference

### `leash init`
```bash
leash init [--force]
```
Initializes the `.leash/` directory in the current repository root with an initial `policy.yaml` configuration. If a policy file already exists, `--force` can be passed to overwrite it.

### `leash run`
```bash
leash run -- <command> [args...]
```
Executes `<command>` wrapped inside a pseudo-terminal:
- Evaluates `.leash/policy.yaml`.
- Enforces `deny` (exit code `126`) or `ask` prompts (`[y/N]`).
- Automatically takes a git tree checkpoint in `refs/leash/checkpoints`.
- Streams all standard input/output/error transparently.
- Logs events to `.leash/log.jsonl`.
- Exits with the exact exit code produced by `<command>`.

### `leash checkpoints`
```bash
leash checkpoints [--session <session_id>]
```
Lists recorded snapshots, including short commit hash, session identifier, timestamp, and description.

### `leash rewind`
```bash
leash rewind <checkpoint_id> [--yes]
```
Restores the working directory to the exact tree recorded at `<checkpoint_id>`:
- Uses forced checkout and cleans untracked files added by the wrapped session.
- **Never changes `HEAD` or user branches**.
- Preserves the `.leash/` audit log and policy configuration.

### `leash log`
```bash
leash log [--session <id>] [-n|--tail <N>] [--json]
```
Displays recorded audit events from `.leash/log.jsonl`. Supports session filtering, tail limits, and raw JSONL export.

---

## Policy Configuration (`.leash/policy.yaml`)

Policies are declared in YAML. Rules are evaluated sequentially from top to bottom; the first rule whose regular expression matches determines the outcome.

```yaml
version: 1

rules:
  # Block destructive filesystem commands
  - name: "block-destructive-rm"
    match: "rm\\s+-rf\\s+(/|~|\\.\\.)"
    action: deny
    reason: "Destructive command targeting sensitive directory"

  # Block raw disk formatting
  - name: "block-mkfs"
    match: "mkfs\\b"
    action: deny
    reason: "Disk format attempt detected"

  # Ask confirmation before force pushing
  - name: "confirm-force-push"
    match: "git\\s+push\\s+.*--force"
    action: ask
    reason: "Force push may overwrite remote history"

  # Ask confirmation for unlisted network tools
  - name: "confirm-curl"
    match: "curl\\s+https?://"
    action: ask
    reason: "Outbound network request requires review"

# Fallback action if no rules match (allow | ask | deny)
default_action: allow
```

### Policy Actions & Non-Interactive Safety
| Action | Interactive Terminal (TTY) | Non-Interactive (CI / Scripts) | Exit Code |
| :--- | :--- | :--- | :--- |
| `allow` | Proceeds immediately | Proceeds immediately | Propagated from command |
| `ask` | Prompts `[y/N]` | **Treated automatically as `deny`** | `126` (blocked) or `1` (aborted) |
| `deny` | Blocked immediately | Blocked immediately | `126` |

> [!IMPORTANT]
> In non-interactive environments (such as CI pipelines or headless scripts without a TTY), `ask` rules automatically fail-safe to `deny` with exit code `126`. This prevents headless automated processes from hanging indefinitely.

---

## Checkpointing Mechanics & Safety Guarantees

Leash implements non-destructive git checkpointing:

1. **Hidden Git Reference (`refs/leash/checkpoints`)**:
   - Checkpoint commits are created directly on `refs/leash/checkpoints`.
   - Your active branch (`main`, `feature`, etc.) and `HEAD` reference are never moved or updated.
   - Works seamlessly even on newly initialized repositories with an unborn `HEAD` (before your initial commit).

2. **Exclusion of `.leash/` Directory**:
   - Checkpoint git trees explicitly exclude `.leash/`.
   - Audit logs (`log.jsonl`) and policy files are not versioned into checkpoints, preventing recursion and repository bloat.

3. **Append-Only Audit Log Preservation**:
   - During `leash rewind`, Leash backs up the local `.leash/` directory before restoring the tree and restores it immediately after checkout.
   - The security audit trail is never deleted or reverted when traveling back in time.

---

## Limitations & Scope (v0.1)

> [!NOTE]
> **Top-Level Command Interception Limitation:**
> In Leash v0.1, declarative policy evaluation is applied strictly to the top-level command passed to `leash run -- <command> [args...]`.
> Subcommands or background processes spawned internally by the AI agent (e.g., inside an interactive subshell) are not intercepted at the OS kernel/system call level in this release. System call / seccomp / eBPF process tree interception is planned for future iterations.

---

## Development & Contributing

Contributions are welcome! To contribute to Leash:

### 1. Set Up Environment
Ensure you have Rust and Git installed:
```bash
cargo --version
git --version
```

### 2. Run Test Suite
Run all unit and integration tests:
```bash
cargo test
```

### 3. Check Code Quality & Formatting
We maintain zero warnings with clippy and strict rustfmt formatting:
```bash
cargo fmt -- --check
cargo clippy -- -D warnings
```

### 4. Submitting Pull Requests
- Keep PRs focused with descriptive commits.
- Ensure all existing tests pass and add new tests for any added features or bug fixes.
- PRs automatically run continuous integration across Ubuntu and macOS.

---

## License

This project is licensed under the [MIT License](LICENSE).

