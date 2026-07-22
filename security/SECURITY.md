# Security Policy

## Reporting a Vulnerability

We take the security of the Lianxi Platform and Trust Gateway seriously.

If you discover a security vulnerability within this repository, please do NOT create a public GitHub issue. Instead, report it directly to our security team via the process described in [`disclosure-policy.md`](disclosure-policy.md).

## Security Model Overview

- **Agents propose. Gateway decides. Executors verify.**
- All execution capabilities require short-lived, Ed25519-signed `ExecutionGrant` tokens.
- Grants are cryptographically bound to canonical JSON input hashes (`input_hash`).
