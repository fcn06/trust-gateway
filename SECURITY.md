# Security Policy

## Reporting Security Issues

We take the security of **Trust Gateway** seriously. If you believe you have discovered a vulnerability, security flaw, or cryptographic weakness, please **do not open a public GitHub issue**.

Instead, report vulnerabilities privately through one of the following channels:

- **Security Advisory Form**: Submit a report via GitHub Private Vulnerability Reporting on this repository.
- **Email**: Reach out directly to `security@lianxi.io` with details of the issue.

### What to Include in Your Report

To help us investigate and resolve issues quickly, please include:
1. **Description**: Clear description of the vulnerability, including affected components or protocol flows.
2. **Reproduction Steps**: Step-by-step instructions, script, or proof-of-concept payload demonstrating the issue.
3. **Impact**: Potential impact of the flaw (e.g., grant forgery, bypass of argument binding, key exposure).
4. **Environment**: Version or commit hash tested, operating system, and deployment topology.

## Security Guarantees & Invariants

Trust Gateway enforces cryptographic and physical execution control boundaries for AI agents. The following invariants are P0 security properties:

1. **No Direct Execution**: Agents never hold standing credentials and cannot execute mutations directly.
2. **Cryptographic Argument Binding**: Every `ExecutionGrant` is bound to a SHA-256 `input_hash` of exact arguments.
3. **Single-Use Nonces**: Grants are single-use (`jti` nonce tracking) and short-lived.
4. **Asymmetric Signing**: In production (`LIANXI_ENV=production`), grants must be signed with Ed25519 keypairs. HMAC symmetric keys are strictly gated to development environments.
5. **Fail-Closed Default**: Undefined tools, invalid signatures, missing policies, or unverified claims result in immediate execution denial.

## Response SLA

- **Initial Response**: Within 48 hours acknowledging receipt of the report.
- **Triage & Status**: Updates provided within 5 business days detailing vulnerability severity and remediation timeline.
- **Fix & Disclosure**: Coordinated disclosure after patches are published.
