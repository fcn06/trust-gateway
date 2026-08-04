# Open Execution Authorization Protocol Specification

Version: 1.0.0

## Overview

The **Open Execution Authorization Protocol** provides a vendor-neutral, cryptographically verifiable specification for controlling side-effecting actions in autonomous AI agent systems.

It separates reasoning intelligence from execution authority by requiring agents to obtain a short-lived, cryptographically signed **ExecutionGrant** before mutating external target systems.

---

## Normative Message Contracts

| Contract | Direction | Purpose |
| :--- | :--- | :--- |
| `ProposedAction` | Agent → Gateway | Intent proposal payload submitted for authorization |
| `PolicyDecision` | Gateway → Internal | Evaluation outcome (`Allow`, `Deny`, `RequireApproval`) |
| `ExecutionGrant` | Gateway → Executor / Agent | Ed25519-signed authorization token bound to exact input digest |
| `GrantedAction` | Gateway → Executor Host | Dispatched execution envelope containing grant and parameters |
| `ExecutionResult` | Executor → Gateway / Agent | Standardized execution outcome status and sanitized output |

---

## `ExecutionGrant` Claims Schema

An `ExecutionGrant` is an Ed25519-signed JWT token containing the following required normative claims:

```json
{
  "iss": "trust-gateway",
  "aud": "executor-host",
  "sub": "agent-001",
  "jti": "grant-550e8400-e29b-41d4-a716-446655440000",
  "exp": 1740000030,
  "nbf": 1740000000,
  "tenant_id": "tenant_demo",
  "tool_id": "io.example.refund@v1",
  "input_hash": "sha256:38c23c59a35e4e7e6a7c1b506692df830c25a7536d506886e3067e2a9b3a1a67",
  "policy_fingerprint": "sha256:8f4e2c1a..."
}
```

### Claim Definitions

* `iss`: Issuer identifier (must equal expected Gateway issuer URI).
* `aud`: Audience identifier (must equal target Executor host domain).
* `sub`: Subject identifier (agent or user initiating proposal).
* `jti`: Unique single-use grant identifier (UUID v4) for replay prevention.
* `exp`: Unix timestamp defining grant expiration (maximum recommended TTL: 30 seconds).
* `nbf`: Unix timestamp before which the grant is invalid.
* `tenant_id`: Multi-tenant isolation context.
* `tool_id`: Fully-qualified identifier of the target tool and version.
* `input_hash`: Hex-encoded SHA-256 digest of RFC 8785 canonicalized input arguments.
* `policy_fingerprint`: Hash of the evaluated policy snapshot for auditing.

---

## Canonicalization & Input Hash Protocol (RFC 8785)

To guarantee argument integrity across heterogeneous components, parameter inputs are canonicalized prior to hashing:

1. Keys in JSON objects are sorted lexicographically by UTF-8 code points.
2. Extra whitespace, indentation, and trailing commas are stripped.
3. Floating point numbers follow standard IEEE 754 canonical formatting.
4. The resulting string is UTF-8 encoded and hashed using SHA-256.

$$\text{input}\_\text{hash} = \text{"sha256:"} + \text{Hex}(\text{SHA256}(\text{Canonicalize}_{\text{RFC8785}}(\text{arguments})))$$

---

## Executor Verification Requirements

An Executor **MUST** reject execution if any of the following conditions fail:

1. **Signature Verification**: The Ed25519 signature fails verification against the Gateway public key.
2. **Class Separation**: Token `typ` or `token_class` is not `execution_grant`.
3. **Expiration**: Current UTC time $> \text{exp}$ or $< \text{nbf}$.
4. **Tool Binding**: `tool_id` in grant does not match requested target tool.
5. **Argument Integrity**: SHA-256 digest of canonicalized execution arguments does not equal `input_hash`.
6. **Replay Check**: `jti` exists in the consumed single-use nonce store.

---

## Replay Prevention Semantics

Grants are strictly single-use. Executors check and record the `jti` in a durable nonce store.
If an incoming `jti` has already been recorded, execution is rejected immediately with error `GRANT_REPLAY_DETECTED`.

---

## Reference Implementation Artifacts

This repository provides reference components implementing this protocol:
* [`gateway/`](../../gateway): Reference policy decision point and grant issuer.
* [`executor_host/`](../../executor_host): Reference execution worker host.
* [`verifier/`](../../verifier): Zero-dependency Rust reference verifier SDK.
* [`conformance/`](../../conformance): Protocol conformance suite.
* [`test-vectors/`](../../test-vectors): Standard test vectors for multi-language implementations.

> **Transport Agnosticism**: NATS JetStream is the default transport of the reference implementation. The protocol specification can be bound to alternative transports (REST/HTTP, gRPC, WebSocket, MCP) without altering grant semantics.

