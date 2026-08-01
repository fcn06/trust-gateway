# Security Guarantees Classification

To ensure complete transparency and avoid security posture misrepresentation, the following table details the technical nature of each guarantee provided by the Lianxi Trust Gateway architecture:

| Guarantee | Nature | Implementation Mechanism | Domain Module |
|---|---|---|---|
| Grant Signature & Audience | **Cryptographic** | Ed25519 asymmetric signature required in production (`LIANXI_ENV=production`), HMAC strictly gated to `LIANXI_ENV=development` with hard boot-time refusal | `crates/trust-grants`, `crates/trust-auth`, `gateway` |
| Input Binding | **Cryptographic** | SHA-256 canonical JSON input hash binding | `crates/trust-canonical`, `crates/trust-executor-sdk` |
| Replay Prevention | **Deterministic** | Single-use JTI tracking via JetStream KV | `crates/trust-grants`, `adapters/storage-nats-kv` |
| Policy Evaluation | **Deterministic** | Priority-ordered attribute-based policy matching | `crates/trust-policy`, `policy-sdk` |
| Human Approval | **Organizational** | WebAuthn biometric approval flow | `gateway/src/approval_daemon.rs` |
| PII Scrubbing / Egress Validation | **Deterministic** | **Primary enforcement**: Executor-side mandatory filtering via `EgressFilter::sanitize_text()` before results leave the execution boundary. **Defense-in-depth**: Gateway-side structural validation of result bounds. | `crates/trust-egress`, `crates/trust-reference-executor`, `executor_host` |
| Audit Trail Integrity | **Cryptographic** | Hash-chained audit event logging | `crates/trust-audit` |
| WASM / Container Isolation | **Technical** | Wasmtime sandbox / OCI container boundaries | `executor_host` |
