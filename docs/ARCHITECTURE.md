# Trust Gateway Architecture

The Trust Gateway enforces **"Agents propose. Gateway decides. Executors verify."**

## Architectural Planes

1. **Reasoning Plane**: Sovereign agents generate proposals (`ProposedAction`).
2. **Governance Plane**: Trust Gateway evaluates attribute-based policy rules (`crates/trust-policy`) and issues short-lived Ed25519 `ExecutionGrant` JWTs (`crates/trust-grants`).
3. **Execution Plane**: Hardened executors (`executor_host`, `crates/trust-reference-executor`) verify grant signatures and SHA-256 `input_hash` binding (`crates/trust-auth`, `crates/trust-executor-sdk`, `verifier`) before running mutations.
4. **Egress Scrubbing Plane**: Results pass through PII scrubbing and structural validation (`crates/trust-egress`) before returning to the agent.
5. **Trust Operations Plane (`trust_ops`)**: Key lifecycle management, executor posture attestation, and verifiable hash-chained audit log reconciliation (`crates/trust-audit`).
