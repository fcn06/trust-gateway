# Trust Gateway Architecture

The Trust Gateway enforces the 3-pillar zero-trust value proposition:
**"Agents propose. Gateway decides. Executors verify."**

---

## 🏛️ The 3 Core Pillars & Architectural Planes

### 🤖 Pillar 1: Agents Propose (Reasoning & Intent)
* **Reasoning Plane**: Sovereign agents generate action intents represented as canonical `ProposedAction` payloads (`crates/trust-model`).
* **Zero-Trust Boundary**: Agents hold zero API credentials, database keys, or direct execution capabilities. All action intents must be submitted over NATS.

---

### 🛡️ Pillar 2: Gateway Decides (Governance & Control)
* **Governance Plane**: The Trust Gateway evaluates request attributes against `policy.toml` (`crates/trust-policy`, `policy-sdk`). If required by policy rules, human approval is triggered via the approval daemon, backed by a durable pending-approvals store.
* **Grant Minting Plane**: Upon approval, the Gateway mints a short-lived Ed25519-signed `ExecutionGrant` JWT (`crates/trust-grants`), cryptographically bound to the SHA-256 `input_hash` of canonical arguments (`crates/trust-canonical`).
* **Trust Operations Plane (`trust_ops`)**: Key lifecycle management (JWKS rotation), executor posture attestation, and hash-chained audit log reconciliation (`crates/trust-audit`).

---

### ⚡ Pillar 3: Executors Verify (Verification & Execution)
* **Execution & Verification Plane**: Isolated runtimes (`executor_host`, `verifier`, `crates/trust-executor-sdk`) verify the Ed25519 signature, SHA-256 `input_hash` argument binding, and single-use replay nonces before dispatching tool mutations.
* **Egress Scrubbing Plane**: Execution results pass through structural validation and PII/secret scrubbing (`crates/trust-egress`) before returning to the agent.

---

## 🧩 Domain Crate Responsibilities

| Pillar | Domain Crate | Responsibility |
| :--- | :--- | :--- |
| **Pillar 1** | [`crates/trust-model`](../crates/trust-model) | Pure domain model definitions (`ProposedAction`, `ExecutionGrant`, `TransactionOutcomeState`). |
| **Pillar 2** | [`crates/trust-policy`](../crates/trust-policy) | Attribute-based policy evaluator engine. |
| **Pillar 2** | [`crates/trust-canonical`](../crates/trust-canonical) | RFC 8785 JSON canonicalizer & SHA-256 `input_hash` digest calculation. |
| **Pillar 2** | [`crates/trust-grants`](../crates/trust-grants) | Ed25519 `ExecutionGrant` issuance and key management. |
| **Pillar 2** | [`crates/trust-audit`](../crates/trust-audit) | Hash-chained `AuditEvent` ledger and audit sinks. |
| **Pillar 3** | [`crates/trust-auth`](../crates/trust-auth) | Strict JWT signature and claims contract verification. |
| **Pillar 3** | [`crates/trust-executor-sdk`](../crates/trust-executor-sdk) | Abstract Executor trait & outcome state reconciliation. |
| **Pillar 3** | [`crates/trust-egress`](../crates/trust-egress) | Regex & LLM-powered PII & secret egress scrubbing. |
| **Pillar 3** | [`verifier/`](../verifier) | Standalone zero-dependency Ed25519 execution grant verifier library. |
