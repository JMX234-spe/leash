# Leash

[![CI](https://github.com/JMX234-spe/leash/actions/workflows/ci.yml/badge.svg)](https://github.com/JMX234-spe/leash/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Leash wraps CLI-based coding agents in a pseudo-terminal (PTY), evaluates declarative security policies, records local JSONL audit logs, and creates automatic git checkpoints for instant rollback. Runs locally with zero telemetry or network calls.

<!-- TODO: Record animated demo (asciinema / GIF) when CLI interface stabilizes -->

```console
$ leash init
Initialized Leash configuration at .leash/policy.yaml

$ leash run -- rm -rf /
[LEASH BLOCKED] Command blocked by policy rule 'block-destructive-rm'
Reason: Destructive command targeting sensitive directory
Command: rm -rf /

$ leash run -- python -c "open('app.py', 'a').write('\ndef apply_discount(): pass\n')"
[LEASH] Created pre-execution checkpoint d4c21e3

$ leash run -- python -c "open('app.py', 'w').write('CORRUPTED SYNTAX'); open('unwanted.tmp', 'w').write('garbage')"
[LEASH] Created pre-execution checkpoint 3a4a4e8

$ leash checkpoints
ID         SESSION    TIMESTAMP                 DESCRIPTION
-------    -------    ------------------------- -----------
3a4a4e8    8e33a4b0   2026-09-11 02:02:24 UTC   before: python -c open('app.py', 'w').write('CORRUPTED SYNTAX'); open('unwanted.tmp', 'w').write('garbage')
d4c21e3    8e33a3b0   2026-09-11 02:02:24 UTC   before: python -c open('app.py', 'a').write('\ndef apply_discount(): pass\n')

$ leash rewind 3a4a4e8 --yes
[LEASH] Successfully rewound working directory to checkpoint 3a4a4e8 (before: python -c open('app.py', 'w').write('CORRUPTED SYNTAX'); open('unwanted.tmp', 'w').write('garbage'))

$ cat app.py
def calculate_total():
    return 100

$ ls unwanted.tmp
ls: cannot access 'unwanted.tmp': No such file or directory
```

## Installation

### Prerequisites
- **Rust toolchain** (Rust 1.75+ or 2021 edition): [rustup.rs](https://rustup.rs)
- **Git** (required for checkpoint backend): [git-scm.com](https://git-scm.com)

### Build and Install from Source

```bash
git clone https://github.com/JMX234-spe/leash.git
cd leash
cargo install --path .
```

Verify the binary is available in your `$PATH`:
```bash
leash --version
```

## Quickstart

Initialize Leash in any Git repository:
```bash
leash init
```

Wrap an AI coding agent or command:
```bash
leash run -- claude "refactor authentication service"
```

List recorded checkpoints:
```bash
leash checkpoints
```

Restore your repository to any pre-execution state:
```bash
leash rewind <checkpoint-id>
```

Inspect local session audit events:
```bash
leash log
```

## How It Works

- **PTY Wrapper**: Leash uses `portable-pty` to spawn commands inside a pseudo-terminal. It passes stdin, stdout, and stderr transparently, preserving terminal features (colors, cursor control, interactive prompts) and propagating the child process exit code upon exit.
- **Policy Engine**: Before spawning the child process, Leash normalizes whitespace and common flag variants (`-r -f`, `--force --recursive` -> `-rf`) and matches the command against regular expressions defined in `.leash/policy.yaml`. Rules evaluate in top-down order; the first match determines the action (`allow`, `ask`, or `deny`).
- **Git Checkpoints**: Prior to running any allowed command, Leash commits the current working tree state to a hidden Git reference (`refs/leash/checkpoints`). The user's active branch and `HEAD` commit remain completely untouched. The `.leash/` directory is excluded from snapshots.
- **Transactional Rewind**: Running `leash rewind` creates an automatic safety backup, backs up `.leash/` locally to `.leash.bak`, executes a forced tree checkout with untracked file removal, updates the Git index, and restores `.leash/`.
- **Audit Logging**: Every session event (start, policy evaluation, checkpoint creation, rewind execution, termination) is recorded in `.leash/log.jsonl`. Sensitive tokens (`sk-...`, `ghp_...`, Bearer tokens, passwords) are automatically redacted prior to being written to disk.

## CLI Reference

| Command | Arguments | Description |
| :--- | :--- | :--- |
| `leash init` | `[--force]` | Initializes `.leash/policy.yaml` with default security rules. |
| `leash run` | `-- <command> [args...]` | Spawns `<command>` in a PTY with policy enforcement and automatic checkpointing. |
| `leash checkpoints` | `[--session <id>]` | Lists all recorded snapshots, optionally filtered by session identifier. |
| `leash rewind` | `<checkpoint-id> [--yes]` | Restores the working directory to the specified checkpoint. |
| `leash log` | `[--session <id>] [-n <count>] [--json]` | Displays recorded session events as a table or raw JSONL. |

## Policy Configuration (`.leash/policy.yaml`)

Policies are declared in YAML. Rules are evaluated sequentially:

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
    reason: "Force push risks overwriting remote repository history"

  - name: "block-unlisted-network"
    match: "curl\\s+http"
    action: ask
    reason: "Network request is not on the allowed list"

default_action: allow
```

### Policy Actions

- `allow`: Command runs immediately.
- `ask`: Prompts the user `[y/N]` before proceeding. In non-interactive environments (without a TTY, such as CI), `ask` rules are automatically treated as `deny` (exit code `126`) to avoid blocking pipelines indefinitely.
- `deny`: Command execution is blocked immediately with exit code `126`.

## Known Limitations (v0.1)

- **Top-Level Command Evaluation**: Policy inspection is performed solely on the top-level command string passed to `leash run -- <cmd>`. Subcommands executed inside subshells, scripts, or child processes spawned by the wrapped program are not intercepted at the OS kernel or syscall level in v0.1.
- **Signal Handling**: Terminal signals (SIGINT, SIGTERM, SIGWINCH) are not yet forwarded down the PTY hierarchy. Pressing `Ctrl+C` will terminate the parent process but may leave child processes running in certain environments.
- **Platform Support**: Primary development and CI targets are Linux and macOS. Windows support relies on ConPTY and is intended for local testing; edge cases around terminal resizing and pseudo-console handling may differ from Unix systems.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, test execution, and code style guidelines.

## License

Leash is released under the [MIT License](LICENSE).
