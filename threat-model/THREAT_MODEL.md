# Trust Gateway Threat Model

This document specifies the security boundaries, assets, threat vectors, and mitigations of the Open Execution Authorization Protocol and Trust Gateway.

## 1. System Assets

1. **Execution Authority**: The ability to issue Ed25519-signed `ExecutionGrant` JWTs.
2. **Business Systems (Executors)**: Databases, SaaS APIs, infrastructure mutation endpoints.
3. **Audit Trails**: Signed append-only action logs and hash chains.

## 2. Threat Matrix

| Threat ID | Vulnerability Vector | Severity | Mitigation Strategy |
|---|---|---|---|
| T-001 | **Grant Re-targeting** (re-using grant for another tool) | Critical | Hard binding of `allowed_action` and `input_hash` verified by executor |
| T-002 | **Grant Replay** (executing token multiple times) | High | Single-use JTI tracking in JetStream KV store |
| T-003 | **Argument Tampering** (modifying tool payload) | High | RFC 8785 Canonical JSON SHA-256 hash comparison |
| T-004 | **Session JWT Substitution** (presenting session token to executor) | High | Strict `aud="executor-host"` validation |
| T-005 | **Executor Spoofing** (unauthorized executor running actions) | High | Executor key registration and attestation level check |

## 3. Trust Boundaries

- **Reasoning Domain (LLM Agents)**: Untrusted. Can only submit action proposals.
- **Governing Domain (Trust Gateway)**: Trusted decision authority. Evaluates 4-layer policies and issues grants.
- **Execution Domain (Executors)**: Isolated capability runners. Must verify grants before executing.
