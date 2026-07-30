# Security Guarantees Classification

To ensure complete transparency and avoid security posture misrepresentation, the following table details the technical nature of each guarantee provided by the Lianxi Trust Gateway architecture:

| Guarantee | Nature | Implementation Mechanism | Domain Module |
|---|---|---|---|
| Grant Signature & Audience | **Cryptographic** | Ed25519 asymmetric signature & strict audience validation | `crates/trust-grants`, `crates/trust-auth` |
| Input Binding | **Cryptographic** | SHA-256 canonical JSON input hash binding | `crates/trust-canonical`, `crates/trust-executor-sdk` |
| Replay Prevention | **Deterministic** | Single-use JTI tracking via JetStream KV | `crates/trust-grants`, `adapters/storage-nats-kv` |
| Policy Evaluation | **Deterministic** | Priority-ordered attribute-based policy matching | `crates/trust-policy`, `policy-sdk` |
| Human Approval | **Organizational** | WebAuthn biometric approval flow | `gateway/src/approval_daemon.rs` |
| PII Scrubbing / Egress Validation | **Deterministic** | Regex PII scrubbing engine and structural response bounds | `crates/trust-egress` |
| Audit Trail Integrity | **Cryptographic** | Hash-chained audit event logging | `crates/trust-audit` |
| WASM / Container Isolation | **Technical** | Wasmtime sandbox / OCI container boundaries | `executor_host` |
