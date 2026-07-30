# Crates — Domain-Driven Security Logic

This directory contains the core domain crates that implement Trust Gateway's security logic. Each crate has a single, focused responsibility and follows hexagonal architecture principles.

## Crate Index

| Crate | Purpose | Key Types |
|---|---|---|
| **[`trust-model`](trust-model/)** | Canonical data models for the execution protocol | `ProposedAction`, `ExecutionGrant`, `GrantedAction`, `OperationAttributes`, `TransactionOutcomeState` |
| **[`trust-canonical`](trust-canonical/)** | Deterministic JSON serialization & SHA-256 `input_hash` computation | `canonical_json()`, `compute_input_hash()` |
| **[`trust-auth`](trust-auth/)** | Scoped JWT signature verification & JWT class isolation | `AuthVerifier`, session vs. grant JWT enforcement |
| **[`trust-policy`](trust-policy/)** | Priority-ordered attribute-based policy evaluation engine | `CorePolicyEngine`, `PolicyDecision` |
| **[`trust-grants`](trust-grants/)** | Ed25519 `ExecutionGrant` JWT minting & replay-nonce tracking | `GrantIssuer`, nonce lifecycle |
| **[`trust-audit`](trust-audit/)** | Hash-chained audit log generator | Append-only audit entries |
| **[`trust-egress`](trust-egress/)** | PII/secret redaction engine & response bounds validator | Regex-based scrubbing rules |
| **[`trust-executor-sdk`](trust-executor-sdk/)** | Abstract `Executor` trait & crash reconciliation | `Executor` trait, `ExecutionResult` |
| **[`trust-reference-executor`](trust-reference-executor/)** | Zero-dependency mock executor for local testing | `ReferenceExecutor` |

## Dependency Graph

```
trust-model (no internal deps)
    ↑
trust-canonical (uses trust-model)
    ↑
trust-policy (uses trust-model)
trust-grants (uses trust-model, trust-canonical)
trust-auth (uses trust-model)
trust-audit (uses trust-model)
trust-egress (uses trust-model)
trust-executor-sdk (uses trust-model, trust-grants)
    ↑
trust-reference-executor (uses trust-executor-sdk, trust-egress)
```

## How to Test

```bash
# Test all domain crates
cargo test --workspace --lib

# Test a specific crate
cargo test -p trust-model --lib
cargo test -p trust-canonical --lib
cargo test -p trust-grants --lib
```

## Security Assumptions

- `trust-canonical` implements RFC 8785 JSON Canonicalization Scheme for deterministic hashing
- `trust-grants` uses Ed25519 signatures — the signing key never leaves the gateway process
- `trust-auth` enforces JWT class separation: session JWTs cannot act as execution grants
- `trust-egress` is the last line of defense — PII scrubbing runs after execution, before returning results
