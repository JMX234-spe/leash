# Security Policy

## Supported Versions

Only the latest release receives security updates.

| Version | Supported          |
| :------ | :----------------- |
| 0.1.x   | :white_check_mark: |
| < 0.1.0 | :x:                |

## Security Model and Scope

Leash is designed as an application-level CLI wrapper and checkpoint engine to protect against accidental damage, unexpected file mutations, and rogue commands issued by automated coding agents.

### What Leash Is
- A transparent PTY wrapper that evaluates top-level CLI commands against declarative regex patterns in `.leash/policy.yaml`.
- An automatic, local git-based snapshot and rewind mechanism (`refs/leash/checkpoints`) to revert unwanted repository modifications.
- An audit logging mechanism that redacts common credential formats before writing to disk.

### What Leash Is Not (Out of Scope)
- **Not an OS sandbox or container runtime**: Leash does not run wrapped processes inside an isolated cgroup, namespace, chroot, or virtual machine.
- **Not a kernel-level syscall filter**: Leash inspects the top-level command string before execution. Commands executed inside child processes, subshells (e.g. `bash -c "rm -rf /"`), or binary executables are not intercepted at the OS kernel or syscall level.
- **Heuristic matching**: Regular expressions provide pattern-matching heuristics. Bypasses involving complex shell string encodings, obfuscation, or alias expansion are treated as policy improvement requests, not critical security vulnerabilities against the Leash binary itself.

If your use case requires untrusted multi-tenant code isolation or isolation against hostile payloads actively attempting kernel breakouts, Leash must be paired with OS-level virtualization (such as Docker, Firecracker, gVisor, or bubblewrap).

## Reporting a Vulnerability

If you discover a security vulnerability within Leash itself (e.g., privilege escalation, local data corruption during checkpoint/rewind, memory safety bugs in native extensions, or insecure credential leakage):

1. **Do not disclose publicly**: Please avoid opening public issues or pull requests.
2. **Private Disclosure**: Report the issue privately via [GitHub Security Advisories](https://github.com/JMX234-spe/leash/security/advisories/new).
3. **Include Details**:
   - Detailed description of the vulnerability.
   - Minimal reproduction script or command sequence.
   - Operating system and environment details.
   - Potential impact and suggested mitigations if available.

### Response Process
- We aim to acknowledge receipt of security reports within 48 hours.
- A private tracking branch will be used to investigate and patch the issue.
- Once a fix is verified and released, a public security advisory will be published crediting the reporter.
