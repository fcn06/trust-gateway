# Public Edge & Sovereign Core Deployment Topology

This document details the multi-server physical deployment topology of the **Trust Gateway**, explaining the architectural split between the **Public Edge Boundary (Server 1)** and the **Sovereign Core Boundary (Server 2)**.

---

## 🏗️ 2-Server Physical Topology

```
[Public Edge Boundary: Server 1]            [Sovereign Core Boundary: Server 2]
┌──────────────────────────────────────┐    ┌──────────────────────────────────────┐
│           TRUST GATEWAY              │    │           TRUST GATEWAY              │
│          (Public Edge)               │    │          (Core Governance)           │
│                                      │    │                                      │
│  platform/global_domain/public_gateway│ ──►│  gateway/ (Policy Engine & Grant)    │
│  platform/tenant_registry            │    │  executor_host/ (Native/VP/Connector) │
│  platform/tenant_context             │    │  crates/ & adapters/                 │
└──────────────────────────────────────┘    └──────────────────────────────────────┘
```

---

## 🛡️ 1. Server 1: Public Edge Boundary (`platform/`)

Server 1 is the stateless, public-facing ingress boundary exposed to external internet clients, webhooks, and browser web applications.

### Security Invariants (Server 1)
- **Zero Stored Credentials**: Server 1 stores no private database keys, user identity records, or SaaS API credentials.
- **Stateless Operation**: If Server 1 is compromised or restarted, no sensitive user data or execution grants can be leaked.

### Core Edge Components
1. **`platform/global_domain/public_gateway`**:
   - **Public Edge Router**: Handles inbound HTTP webhooks, DID resolution requests (`did:web`, `did:twin`), and public API endpoints.
   - **Secure WebSocket Proxy (`/api/ws`)**: Relays real-time audit and status events from internal NATS streams to client browser applications without exposing internal NATS ports (port 9222 / 4222) to the public internet.
   - **XChaCha20-Poly1305 / HKDF Encryption**: Encrypts and decrypts ephemeral JIT routing tokens for leaf node peering.

2. **`platform/tenant_registry`**:
   - **Multi-Tenant Router**: Maps incoming DID identities and request origins to workspace tenant IDs dynamically.

3. **`platform/tenant_context`**:
   - **Context Provider**: Holds lightweight tenant metadata and context definitions.

---

## 🔒 2. Server 2: Sovereign Core Boundary (`gateway/` & `executor_host/`)

Server 2 is the private, hardened execution control plane housing the policy engine, key vault, and sandboxed executor workers.

### Security Invariants (Server 2)
- **Private Network Isolation**: Server 2 is isolated from direct public internet ingress. Communication from Server 1 flows exclusively over NATS leaf node peering (port 7422) protected by mTLS / Ed25519 NKey authentication.
- **Execution Capability Isolation**: SaaS API credentials and execution scripts exist only on Server 2 inside isolated executor profiles.

### Core Governance Components
1. **`gateway/`**:
   - **Attribute Policy Evaluator (`crates/trust-policy`)**: Evaluates `ProposedAction` payloads against priority-ordered `policy.toml` rules.
   - **Grant Minter (`crates/trust-grants`)**: Mints short-lived (30s-60s) Ed25519-signed `ExecutionGrant` JWTs bound to canonical SHA-256 `input_hash` digests.
   - **Human-in-the-Loop Approval Daemon**: Manages pending approvals via NATS KV (`action_reviews` bucket and `gateway.v1.approval.decision` topic).

2. **`executor_host/`**:
   - **Unified Executor Runtime**: Dispatches execution workloads under dedicated profiles (`native-tool`, `connector`, `vp`).
   - **Cryptographic Grant Verification (`crates/trust-auth`)**: Asserts `aud = "executor-host"`, verifies Ed25519 signature, recalculates SHA-256 `input_hash`, and enforces single-use replay nonces.

3. **Domain Crates (`crates/`) & Adapters (`adapters/`)**:
   - Modular, zero-dependency core crates (`trust-model`, `trust-canonical`, `trust-audit`, `trust-egress`) and storage adapters (`transport-nats`, `storage-nats-kv`).

---

## 📡 3. Cross-Server Communication (Leaf Node Peering)

The public edge and sovereign core communicate over a NATS Leaf Node bridge:
- **Port 7422**: Cross-server pub/sub of namespaced messages (`trust.v1.<tenant>.action.propose`, `exec.v1.<tenant>.<profile>.invoke`).
- **State Isolation**: Leaf node peering strictly permits message passing while blocking JetStream bucket state synchronization across the public boundary.
