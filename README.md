# 🛡️ Trust Gateway

[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue)](LICENSE)
[![NATS](https://img.shields.io/badge/NATS-JetStream-green?logo=nats.io)](https://nats.io)
[![MCP](https://img.shields.io/badge/MCP-SSE%20%2B%20Streamable-purple)](https://modelcontextprotocol.io)
[![Quick Start](https://img.shields.io/badge/Quick%20Start-3%20commands-brightgreen)](#quickstart-wire-claude-in-3-lines)


> **Zero-Trust Governance & Execution Control Plane for Autonomous AI Agents**

`trust-gateway` is an open-source, technology-neutral execution authorization engine. It decouples an AI Agent's **intent to act** from the **authority to execute mutations** against real-world APIs, SaaS tools, and local execution environments.

---

## 🌟 Core Architecture

> **"Agents propose. Gateway decides. Executors verify."**

The system enforces a 5-step zero-trust execution control flow:

$$\text{Agent Intent (ProposedAction)} \longrightarrow \text{Policy Engine} \longrightarrow \text{ExecutionGrant JWT} \longrightarrow \text{Executor (verify input-hash)} \longrightarrow \text{Sanitized Result (PII Filter)}$$

```
                   ┌──────────────────────────────────────┐
                   │             AI AGENT                 │
                   └──────────────────┬───────────────────┘
                                      │ 1. Propose Action
                                      ▼
                   ┌──────────────────────────────────────┐
                   │           TRUST GATEWAY              │
                   │  - Attribute Policy Evaluator        │
                   │  - Short-Lived Ed25519 Grant Issuer  │
                   └──────────────────┬───────────────────┘
                                      │ 2. Signed ExecutionGrant JWT
                                      ▼
                   ┌──────────────────────────────────────┐
                   │            EXECUTOR HOST             │
                   │  - Verify Ed25519 Grant Signature    │
                   │  - Verify SHA-256 input_hash         │
                   │  - Check Single-Use Nonces           │
                   └──────────────────┬───────────────────┘
                                      │ 3. PII Sanitized Result
                                      ▼
                   ┌──────────────────────────────────────┐
                   │              SAAS / API              │
                   └──────────────────────────────────────┘
```

---

## ⚡ Quickstart (Under 1 Minute)

Run the zero-dependency standalone control flow example:

```bash
cd trust-gateway
cargo run -p quickstart-standalone
```

### Running Tests

```bash
cargo test --workspace --lib
```

---

## 📄 Governance Policy (`policy.toml`)

```toml
[governance]
policy_version = "1.0.0"
default_action = "deny"

[[rules]]
name = "Auto-allow read-only queries"
operation_kind = "read_only"
action = "allow"
priority = 10

[[rules]]
name = "Require approval for financial operations"
operation_kind = "financial_mutation"
action = "require_approval"
min_amount_cents = 10000
priority = 20
```

---

## 📜 Security Invariants

1. **No Direct Execution**: Agents never execute actions directly; all mutations require an `ExecutionGrant`.
2. **Cryptographic Binding**: Grants contain SHA-256 `input_hash` of exact arguments to prevent post-approval parameter tampering.
3. **Single-Use Replay Prevention**: Nonce tracking guarantees a grant cannot be executed twice.
4. **JWT Class Separation**: Session JWTs are strictly prohibited from acting as Execution Grants.
5. **Fail-Closed Default**: Unrecognized tools or missing policies default to `deny` or `read_only`.

---

## 📚 Documentation

The [`docs/`](docs/) directory contains in-depth architectural and technical specifications:

- **[`docs/DEPLOYMENT_TOPOLOGY.md`](docs/DEPLOYMENT_TOPOLOGY.md)**: Public Edge (Server 1) vs Sovereign Core (Server 2) physical deployment topology detailing stateless public edge ingress (`platform/`) and private governance execution (`gateway/` & `executor_host/`).
- **[`docs/EXECUTION_EXAMPLE.md`](docs/EXECUTION_EXAMPLE.md)**: Real-World Execution Flow Example detailing how `@agent Inspect the schema of the "sales" dataset.` flows through proposal, policy evaluation, human approval, grant minting, sandboxed execution, and PII egress scrubbing.
- **[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)**: Architectural plane breakdown (Reasoning, Governance, Execution, Egress Scrubbing, and Trust Operations) and domain crate responsibilities.
- **[`docs/QUICKSTART.md`](docs/QUICKSTART.md)**: Detailed workspace compilation, zero-dependency standalone control flow execution, and CLI audit verification commands.
- **[`docs/PROTOCOL_SPEC.md`](docs/PROTOCOL_SPEC.md)**: Open Execution Authorization Protocol specification detailing domain model contracts (`crates/trust-model`), RFC 8785 JSON canonicalization & SHA-256 `input_hash` calculation (`crates/trust-canonical`), and schema contracts.
- **[`docs/NATS_TOPOLOGY.md`](docs/NATS_TOPOLOGY.md)**: Network topology, NATS subject routing matrix, leaf node peering between Public Edge and Sovereign Core, and JetStream storage key rules.
- **[`docs/security-guarantees.md`](docs/security-guarantees.md)**: Security Guarantees classification matrix detailing Cryptographic, Deterministic, and Technical security properties mapped to their underlying domain modules.

---

## 📦 Workspace Structure

### Core Domain Logic & Technology Adapters
- **`crates/` (Domain-Driven Security Logic)**:
  - `trust-model`: Canonical data models (`ProposedAction`, `ExecutionGrant`, `TransactionOutcomeState`, `OperationAttributes`).
  - `trust-canonical`: Deterministic JSON key sorting & SHA-256 `input_hash` calculation.
  - `trust-auth`: Scoped JWT signature verifiers & class isolation.
  - `trust-policy`: Priority-ordered attribute-based policy evaluation engine.
  - `trust-grants`: Ed25519 `ExecutionGrant` minting & replay-nonce tracking.
  - `trust-audit`: Hash-chained audit log generator.
  - `trust-egress`: PII redacting regex scrubbing engine & response bounds validator.
  - `trust-executor-sdk`: Abstract `Executor` trait & crash reconciliation handler.
  - `trust-reference-executor`: Zero-dependency mock executor for local testing.

- **`adapters/` (Technology Transports & Storage)**:
  - `transport-nats`: Decoupled NATS pub/sub message router.
  - `storage-nats-kv`: NATS JetStream key-value state adapter.

### Control Plane Executables & Routers
- **`gateway/`**: Control plane daemon binary (main router, policy evaluator, and approval daemon).
- **`executor_host/`**: Hardened execution runtime daemon dispatching execution profiles (`native-tool`, `connector`, `vp`).
- **`platform/`**: Edge routing infrastructure:
  - `global_domain/public_gateway`: Ingress edge router bridging A2A requests over NATS.
  - `tenant_registry`: Directory store mapping public DID identities to workspace tenants.
  - `tenant_context`: Multi-tenant credentials schemas and configuration metadata.
- **`shared_libs/`**: Facade libraries re-exporting domain crates (`trust_core`, `trust_policy`, `trust_auth`, `identity_context`, `community_adapters`, `ssi_crypto`).
- **`connector_mcp_server/`**: Standalone HTTP OAuth2 callback redirect server (port 3050).
- **`vp_mcp_server/`**: VP MCP server resolving decentralized credentials (port 3051).

### Tools, Testing & Specifications
- **`native_tools/`**: Sandboxed shell and python execution scripts (`inspect_schema`, `compute_statistics`, `detect_anomalies`, `generate_markdown`, `join_datasets`, `sample_rows`, `claw_weather`, etc.).
- **`native_tool_executor/`**: Legacy standalone tool executor crate (deprecated; execution is handled by `executor_host --profile native-tool`).
- **`verifier/`**: Zero-dependency standalone Ed25519 execution grant verification crate.
- **`policy-sdk/`**: Policy rules parser and validation SDK.
- **`trust_ops/`**: Operational utilities and administrative tools.
- **`trustctl/`**: CLI management utility (`policy lint`, `policy simulate`, `audit verify`).
- **`conformance/`**: Test suite runner for security invariants and grant vector verification.
- **`examples/`**: Standalone quickstart demonstration (`quickstart_standalone`).
- **`protocol/`**: Protocol specification documents for A2A and execution grant formats.
- **`security/`**: Security policies, threat assessments, and security invariants.
- **`threat-model/`**: Threat modeling diagrams and attack surface analysis.
- **`docs/`**: Architecture specifications and API reference guides.
- **`test-vectors/`**: JSON test vector files for grant verification and input binding.
- **`tests/`**: Integration and regression test suites.
- **`config/`**: Deployment configuration files and policy templates (`policy.standalone.toml`).
- **`deploy/`**: Docker Compose (`docker-compose.yml`) and systemd deployment assets.
