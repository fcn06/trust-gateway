# Public Edge & Sovereign Core Deployment Model

This document describes, at a conceptual level, the recommended deployment separation for **Trust Gateway**: a **stateless public edge** boundary and a **private governance core** boundary.

---

## 🏗️ Deployment Principle

Trust Gateway is designed to run with a hard boundary between:

- A **stateless public edge** — the only surface exposed to external clients, holding no credentials, keys, or execution authority.
- A **private governance core** — the policy engine, grant issuer, and executors, isolated from direct public internet ingress.

```
   [ Public-Facing Edge ]                 [ Private Governance Core ]
   Stateless ingress            ──────►    Policy Engine
   No stored credentials                   Grant Issuance
                                            Executor Runtime
```

The exact internal network topology (server counts, port assignments, and inter-node routing) is an operational deployment detail left to each deployer and is intentionally not prescribed here — see `deploy/` for a reference Docker Compose setup you can adapt.

---

## 🛡️ 1. Public Edge Boundary

The public edge is the stateless, internet-facing ingress layer handling inbound requests, webhook callbacks, and DID resolution (standard `did:web` and similar methods).

### Security Invariants
- **Zero Stored Credentials**: the public edge stores no private keys, identity records, or SaaS API credentials.
- **Stateless Operation**: if the public edge is compromised or restarted, no sensitive data or execution grants can be leaked from it.
- Real-time status/audit events are relayed to browser clients via a proxy layer without exposing internal message-bus endpoints publicly.

---

## 🔒 2. Governance Core Boundary

The governance core is the private, hardened control plane housing the policy engine, grant issuer, and sandboxed executor workers (`gateway/`, `executor_host/`, and the domain `crates/`).

### Security Invariants
- **Private Network Isolation**: the governance core is not directly reachable from the public internet; all inbound traffic from the edge is authenticated and encrypted in transit.
- **Execution Capability Isolation**: SaaS credentials and execution scripts exist only inside isolated executor profiles within this boundary, never on the edge.

### Core Governance Responsibilities
1. **`gateway/`** — evaluates `ProposedAction` payloads against `policy.toml` rules (`crates/trust-policy`), mints short-lived Ed25519-signed `ExecutionGrant`s bound to a canonical `input_hash` (`crates/trust-grants`), and manages human-in-the-loop approvals for high-risk actions.
2. **`executor_host/`** — verifies each grant's signature, `input_hash` binding, and single-use replay nonce before dispatching the underlying tool (`native-tool`, `connector`, `vp` profiles), via `crates/trust-auth`.
3. **Domain crates & adapters** — the zero-dependency core logic (`trust-model`, `trust-canonical`, `trust-audit`, `trust-egress`) plus transport/storage adapters, all decoupled from any specific deployment topology.

---

## 📡 3. Edge ↔ Core Communication

Communication between the public edge and the governance core is authenticated, encrypted, and strictly scoped:
- Only a narrow, allow-listed set of message subjects may cross the boundary — internal governance and audit subjects never traverse the public edge.
- State (KV buckets, JetStream streams) is never synchronized across the boundary — only message passing is permitted.

Deployers wiring up their own infrastructure should define their own network ACLs and port allocations appropriate to their environment; see `deploy/` for a starting reference configuration.
