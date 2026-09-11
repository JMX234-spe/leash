# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Truncate long command descriptions in `leash checkpoints` table output.

## [0.1.0] - 2026-09-11

### Added
- **Pseudo-Terminal Wrapper**: Transparent process execution via `portable-pty`, preserving ANSI terminal sequences, interactive input, and propagating child process exit codes.
- **Declarative Policy Engine**: Regex-based policy evaluation configured via `.leash/policy.yaml` with top-down rule evaluation and actions (`allow`, `ask`, `deny`).
- **Command Normalization**: Whitespace collapse and destructive flag normalization (e.g. `-r -f`, `--force --recursive` normalized to `-rf`) to prevent naive pattern bypasses.
- **Non-Interactive CI Guard**: Automatic promotion of `ask` rules to `deny` (exit code `126`) when running in headless environments without an interactive TTY.
- **Git Checkpoint Backend**: Snapshot engine using `git2`, storing working tree states in hidden references (`refs/leash/checkpoints`) without disturbing active branches or `HEAD`.
- **Pre-execution Safeguards**: Automatic detection of unborn branches and non-git directories with actionable error messages.
- **Transactional Rewind**: Repository rollback with forced tree checkout, untracked file removal, automatic pre-rewind safety snapshots, and atomic `.leash.bak` backup protection.
- **Structured Audit Logging**: Local event logging in `.leash/log.jsonl` recording session lifecycle, policy evaluations, checkpoints, and rewinds.
- **Automated Credential Redaction**: Real-time sanitization of sensitive tokens (`sk-...`, `ghp_...`, `Authorization: Bearer`, CLI password flags) before writing audit events to disk.
- **CLI Commands**:
  - `leash init`: Initialize `.leash/policy.yaml` with default security rules.
  - `leash run`: Spawn arbitrary commands inside a supervised PTY session.
  - `leash checkpoints`: Inspect recorded checkpoints with optional session filtering.
  - `leash rewind`: Restore repository working tree to a designated checkpoint.
  - `leash log`: Query recorded audit events as formatted tables or raw JSONL.
