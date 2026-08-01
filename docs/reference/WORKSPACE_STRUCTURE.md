# 📦 Workspace Structure

## Core Domain Logic & Technology Adapters
- **[`crates/`](../../crates/)** (Domain-Driven Security Logic):
  - `trust-model`: Canonical data models (`ProposedAction`, `ExecutionGrant`, `TransactionOutcomeState`, `OperationAttributes`).
  - `trust-canonical`: Deterministic JSON key sorting & SHA-256 `input_hash` calculation.
  - `trust-auth`: Scoped JWT signature verifiers & class isolation.
  - `trust-policy`: Priority-ordered attribute-based policy evaluation engine.
  - `trust-grants`: Ed25519 `ExecutionGrant` minting & replay-nonce tracking.
  - `trust-audit`: Hash-chained audit log generator.
  - `trust-egress`: PII redacting regex scrubbing engine & response bounds validator.
  - `trust-executor-sdk`: Abstract `Executor` trait & crash reconciliation handler.
  - `trust-reference-executor`: Zero-dependency mock executor for local testing.

- **[`adapters/`](../../adapters/)** (Technology Transports & Storage):
  - `transport-nats`: Decoupled NATS pub/sub message router.
  - `storage-nats-kv`: NATS JetStream key-value state adapter.

## Control Plane Executables & Routers
- **[`gateway/`](../../gateway/)**: Control plane daemon binary (main router, policy evaluator, and approval daemon).
- **[`executor_host/`](../../executor_host/)**: Hardened execution runtime daemon dispatching execution profiles (`native-tool`, `connector`, `vp`).
- **[`platform/`](../../platform/)**: Edge routing infrastructure:
  - `global_domain/public_gateway`: Ingress edge router bridging A2A requests over NATS.
  - `tenant_registry`: Directory store mapping public DID identities to workspace tenants.
  - `tenant_context`: Multi-tenant credentials schemas and configuration metadata.
- **[`shared_libs/`](../../shared_libs/)**: Facade libraries re-exporting domain crates (`trust_core`, `trust_policy`, `trust_auth`).
- **[`connector_mcp_server/`](../../connector_mcp_server/)**: Standalone HTTP OAuth2 callback redirect server.

## Tools, Testing & Specifications
- **[`native_tools/`](../../native_tools/)**: Hardened native shell and Python tools executed by the native-tool executor profile (`env_clear`, process group isolation, resource limits).
- **[`verifier/`](../../verifier/)**: Zero-dependency standalone Ed25519 execution grant verification crate.
- **[`policy-sdk/`](../../policy-sdk/)**: Policy rules parser and validation SDK.
- **[`trust_ops/`](../../trust_ops/)**: Operational utilities and administrative tools.
- **[`trustctl/`](../../trustctl/)**: CLI management utility (`policy lint`, `policy simulate`, `audit verify`).
- **[`conformance/`](../../conformance/)**: Test suite runner for security invariants and grant vector verification.
- **[`examples/`](../../examples/)**: Standalone quickstart, REST, Python, and Kubernetes deployment examples.
- **[`protocol/`](../../protocol/)**: Protocol specification documents for A2A and execution grant formats.
- **[`security/`](../../security/)**: Security policies, threat assessments, and security invariants.
- **[`threat-model/`](../../threat-model/)**: Threat modeling diagrams and attack surface analysis.
- **[`test-vectors/`](../../test-vectors/)**: JSON test vector files for grant verification and input binding.
- **[`tests/`](../../tests/)**: Integration and regression test suites.
- **[`config/`](../../config/)**: Deployment configuration files and policy templates.
- **[`deploy/`](../../deploy/)**: Docker Compose and deployment assets.
