# Contributing to Leash

Guidelines for development setup, testing, code style, and submitting pull requests.

## Development Setup

### Prerequisites
- **Rust Toolchain**: Stable compiler (Rust 1.75+ or 2021 edition). Install via [rustup](https://rustup.rs).
- **Git**: Git 2.20+ installed and available in `$PATH`.
- **Operating System**: Linux, macOS, or Windows (Windows requires ConPTY support).

### Building

Clone the repository and build in debug mode:

```bash
git clone https://github.com/JMX234-spe/leash.git
cd leash
cargo build
```

To build an optimized release binary:

```bash
cargo build --release
```

## Running Verification

All contributions must pass the standard verification suite prior to submission. CI enforces zero warnings and zero test failures.

### 1. Test Suite

Run the full unit and integration test suite:

```bash
cargo test
```

For verbose test output:

```bash
cargo test -- --nocapture
```

### 2. Linting (Clippy)

Clippy must pass with zero warnings across all targets:

```bash
cargo clippy --all-targets -- -D warnings
```

### 3. Formatting (rustfmt)

Code formatting must adhere to the standard Rust style:

```bash
cargo fmt -- --check
```

To automatically format your code:

```bash
cargo fmt
```

## Architecture Overview

The codebase is organized into modular components in `src/`:

- `src/main.rs`: Application entry point, CLI dispatch, and subcommand handlers (`handle_init`, `handle_run`, `handle_checkpoints`, `handle_rewind`, `handle_log`).
- `src/cli.rs`: CLI argument parser definitions using `clap` (`Cli`, `Commands`, and subcommand argument structures).
- `src/checkpoint/`: Git-based snapshot engine using `git2`:
  - `src/checkpoint/mod.rs`: Re-exports `GitCheckpointBackend` and checkpoint types.
  - `src/checkpoint/git_backend.rs`: Checkpoint creation, tree snapshot generation, reference management under `refs/leash/checkpoints`, pre-rewind safety backups, and transactional rollback.
- `src/policy/`: Declarative security policy engine:
  - `src/policy/mod.rs`: Module interface and re-exports.
  - `src/policy/schema.rs`: YAML serialization models (`PolicyConfig`, `PolicyRule`, `PolicyAction`).
  - `src/policy/engine.rs`: Regex compilation, command normalization, and policy evaluation logic.
- `src/pty_wrapper.rs`: Cross-platform pseudo-terminal process execution via `portable-pty`.
- `src/session_log.rs`: Structured event audit logger (`.leash/log.jsonl`) with automated secret/credential redaction.
- `src/config.rs`: Discovery, default configuration generation, and loading of `.leash/policy.yaml`.

## Pull Request Guidelines

1. **Branch off `main`**: Create a feature or fix branch from `main` (`git checkout -b fix/issue-description`).
2. **Atomic Changes**: Keep changes focused on a single concern. Avoid bundling unrelated refactors or formatting changes.
3. **Tests Required**: Any logic modification or bug fix must include corresponding unit or integration tests in `tests/` or inline `#[cfg(test)]` modules.
4. **Pass All Checks**: Verify `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt -- --check` locally before opening a pull request.
5. **Conventional Commits**: Format commit messages following conventional commit specifications:
   - `feat: add support for custom git ref prefixes`
   - `fix: handle missing pty streams on windows termination`
   - `docs: clarify non-interactive policy behavior`
   - `refactor: extract checkout builder helper in checkpoints`
   - `test: add regression test for unquoted flag normalization`
