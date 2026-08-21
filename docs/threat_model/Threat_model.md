# Trust Gateway Threat Model & Security Architecture

This document provides a formal, comprehensive threat model for the **Lianxi Trust Gateway** and the **Open Execution Authorization Protocol (OEAP)**. It defines the system trust boundaries, security assets, STRIDE threat taxonomy, failure recovery modes, and mitigation invariants.

---

## 1. Governing Principle & System Architecture

The Trust Gateway enforces a strict three-way architectural decoupling:

$$\text{Agent Proposes} \quad \longrightarrow \quad \text{Gateway Decides} \quad \longrightarrow \quad \text{Executors Verify}$$

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                                   REASONING DOMAIN                                     │
│  [ Untrusted AI Agent / LLM / Swarm ]                                                  │
│  - Proposes tool invocation payloads                                                   │
│  - Has ZERO credentials for downstream SaaS/databases                                  │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │ 1. Propose Action (NATS/HTTP/MCP)
                                            ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                                  GOVERNING DOMAIN                                      │
│  [ Trust Gateway Control Plane (Policy Engine) ]                                       │
│  - 4-Layer Monotonic Policy Evaluation (Platform, Org, Agent, Transaction)             │
│  - Agent Registry Kill Switch & Revocation Verification                                │
│  - Mints Short-Lived Ed25519 ExecutionGrant JWTs                                       │
│  - Cryptographically binds: tenant_id + workspace_id + tool + canonical input_hash     │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │ 2. Dispatch GrantedAction (NATS)
                                            ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                                  EXECUTION DOMAIN                                      │
│  [ Isolated Executor Host & Sandboxes ]                                                │
│  - Verifies Ed25519 signature, tenant/workspace scope, and canonical input_hash        │
│  - Atomically consumes single-use replay nonce in JetStream KV                         │
│  - Holds target credentials and executes mutations against downstream providers        │
│  - Uses deterministic Provider Idempotency Keys to safeguard crash recovery            │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │ 3. Execute Mutation with Idempotency Key
                                            ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                               EXTERNAL SAAS / DATABASES                                │
│  [ Stripe, GitHub, Salesforce, PostgreSQL, AWS KMS, etc. ]                             │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Trust Boundaries

| Domain | Trust Level | Capabilities & Constraints |
|---|---|---|
| **Reasoning Domain (Agents)** | **Untrusted** | Generates proposed actions and natural language plans. Cannot access external mutation APIs, execute tools directly, or mint authorization tokens. |
| **Governing Domain (Trust Gateway)** | **High Trust (Decision Authority)** | Evaluates policies against verified actor identity context, manages approval workflows, and signs short-lived `ExecutionGrant` JWTs with Ed25519 keys. |
| **Execution Domain (Executors)** | **Medium-High Trust (Capability Runners)** | Isolated environments with SaaS API credentials. Must strictly verify `ExecutionGrant` tokens, validate argument hashes, enforce tenant/workspace boundaries, and check single-use nonces before executing mutations. |
| **External Providers (SaaS/APIs)** | **External Boundary** | Downstream target systems that accept state mutations. Interacted with using deterministic provider idempotency keys (`idemp_{tenant}_{workspace}_{grant_id}`). |

---

## 3. Core Security Assets

1. **Grant Signing Authority (Ed25519 Private Key)**: Root cryptographic key used to mint `ExecutionGrant` JWTs.
2. **SaaS & Infrastructure Credentials**: API keys, OAuth tokens, and database passwords held exclusively in the Execution Domain.
3. **Tenant & Workspace Compartments**: Cryptographic and subject-level partitions isolating multi-tenant data, policies, and sandbox environments.
4. **Downstream Mutation Integrity**: Guarantee that external financial, database, or infrastructure mutations occur exactly once without duplicate side-effects.
5. **Append-Only Audit Trails**: Tamper-evident, hash-chained transaction logs and audit events in JetStream streams.

---

## 4. STRIDE Threat Matrix & Mitigations

| Threat ID | Category | Threat Scenario | Impact | Severity | Mitigation Strategy |
|---|---|---|---|---|---|
| **T-001** | **Elevation of Privilege** | **Cross-Context Grant Smuggling**: A valid grant minted in a relaxed development/sandbox workspace is replayed in a production workspace or across tenants. | Critical | P0 | Dual binding of `tenant_id` and `workspace_id` in the signed JWT claims. Executors verify: `grant.tenant_id == context.tenant_id && grant.workspace_id == context.workspace_id`. |
| **T-002** | **Tampering / Repudiation** | **Post-Execution Crash Desynchronization**: Gateway/executor crashes after the downstream API accepts the mutation but before the local receipt is committed. | High | P0 | Deterministic `provider_idempotency_key` derived as `idemp_{tenant}_{workspace}_{grant_id}`. Reconciler queries downstream state on retry before replaying mutations. |
| **T-003** | **Tampering** | **Tool Argument Tampering**: An attacker intercepts a valid grant and modifies parameters (e.g. changing refund amount or recipient DID). | High | P0 | RFC 8785 Canonical JSON SHA-256 `input_hash` cryptographically signed in the grant. Executor re-computes argument hash and rejects on mismatch. |
| **T-004** | **Replay / Spoofing** | **Grant Token Replay**: Re-submitting a captured `ExecutionGrant` multiple times to trigger duplicate mutations. | High | P0 | Atomic single-use nonce consumption in NATS KV (`grant_nonces`) with short TTL (30s–60s). Second presentation is rejected. |
| **T-005** | **Elevation of Privilege** | **Confused Deputy / Token Substitution**: Presenting a long-lived session JWT directly to an executor in place of an `ExecutionGrant`. | High | P0 | Strict JWT class separation: `aud="executor-host"`, `iss="trust_gateway"`, and typed verification via `trust_verifier`. Session JWTs are rejected. |
| **T-006** | **Information Disclosure** | **PII & Data Egress Exfiltration**: Tool responses containing sensitive customer PII or credentials leaking back to reasoning agents. | High | P1 | Deterministic egress pipeline: PII regex scrubbing + deterministic schema verification before results are returned to the caller. |
| **T-007** | **Denial of Service / Elevation** | **Compromised Agent Bypass**: An agent marked as revoked or killed continues executing actions. | High | P0 | Central Agent Registry check runs *before* policy evaluation and grant minting. Revoked/killed agents fail closed immediately. |

---

## 5. Deep-Dive: Grant ID Idempotency & Crash Recovery

### The Failure Mode
If an executor submits a mutation (e.g., charge a credit card via Stripe, provision a cloud server) and the process crashes before writing the outcome to durable storage:
- A naive retry could re-execute the mutation, resulting in duplicate billing or resource creation.
- A naive nonce rejection would mark the operation as failed, leaving the system in an inconsistent state where external side-effects occurred without an internal receipt.

### The Recovery Solution

```text
       Executor                        Downstream SaaS                  JetStream KV
          │                                   │                              │
          │ 1. Execute(provider_idemp_key)    │                              │
          ├──────────────────────────────────►│                              │
          │    [200 OK: Side-effect executed] │                              │
          │◄──────────────────────────────────┤                              │
          │                                   │                              │
          💥 CRASH! (Before storing receipt)   │                              │
          ─────────────────────────────────────                               │
          │                                                                  │
      [RECOVERY / RETRY]                                                     │
          │                                                                  │
          │ 2. Check Idempotency Cache (action_id / grant_id)                │
          ├─────────────────────────────────────────────────────────────────►│
          │◄─────────────────────────────────────────────────────────────────┤ (miss/ambiguous)
          │                                   │                              │
          │ 3. Reconcile with provider_idemp_key                             │
          ├──────────────────────────────────►│                              │
          │◄──────────────────────────────────┤ [Returns original receipt]   │
          │                                   │                              │
          │ 4. Store Reconciled Receipt       │                              │
          ├─────────────────────────────────────────────────────────────────►│
          │                                                                  │
```

1. **Deterministic Idempotency Key**:
   $$\text{provider\_idempotency\_key} = \text{"idemp\_"} + \text{tenant\_id} + \text{"\_"} + \text{workspace\_id} + \text{"\_"} + \text{grant\_id}$$
   Strictly formatted with underscore `_` separators per **INV-009**.

2. **Transaction State Machine**:
   $$\text{Proposed} \longrightarrow \text{Authorized} \longrightarrow \text{ExecutionStarted} \longrightarrow \text{SideEffectConfirmed} \longrightarrow \text{ResultPersisted}$$
   If a crash occurs in $\text{ExecutionStarted}$, the recovery scanner marks the transaction as $\text{ReconciliationRequired}$ and uses the $\text{provider\_idempotency\_key}$ to confirm downstream status without re-executing.

---

## 6. Deep-Dive: Tenant and Workspace Boundary Binding

### The Context-Smuggling Vector
In multi-tenant, multi-workspace enterprise deployments:
- Workspace A: `workspace_id = "sandbox-01"` (relaxed development policy)
- Workspace B: `workspace_id = "prod-01"` (strict multi-signature financial policy)

If grants were only bound to `tenant_id`, an attacker could obtain an auto-approved grant in `sandbox-01` and inject it into the `prod-01` executor queue.

### The Cryptographic Fix
1. **JWT Claims Envelope**:
   ```json
   {
     "jti": "grant_9f3b7c2a-1122",
     "action_id": "act_884920",
     "tenant_id": "tenant_enterprise_01",
     "workspace_id": "ws_production_core",
     "allowed_action": "stripe.customer.refund",
     "input_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
     "clearance": "human_approved",
     "exp": 1771660830,
     "aud": "executor-host",
     "iss": "trust_gateway"
   }
   ```
2. **Executor Ingress Enforcement**:
   ```rust
   grant_validator.validate_bound_context(
       &token,
       &tool_id,
       &canonical_args,
       Some(&envelope.tenant_id),
       Some(&envelope.workspace_id),
   ).await?;
   ```
   Any mismatch between the execution queue context and the signed grant claims immediately aborts execution.

---

## 7. Security Invariants Traceability Matrix

| Invariant ID | Title | Rule Reference | Threat Matrix Link |
|---|---|---|---|
| **INV-001** | Zero Direct Tool Execution | `.agents/rules/` | T-005 (Enforces Gateway gating) |
| **INV-002** | JWT Contracts | `.agents/rules/010_JWT_CONTRACTS.md` | T-005 (Validated JWT types only) |
| **INV-003** | No SaaS Credentials in Agents | `.agents/meta/10-gold-okf/invariants/` | T-005 (Executors isolate secrets) |
| **INV-004** | Executor-Only Mutation Access | `.agents/meta/10-gold-okf/invariants/` | T-001, T-003 |
| **INV-005** | Grant Nonce Single-Use | `.agents/meta/10-gold-okf/invariants/` | T-004 (Atomic JTI consumption) |
| **INV-006** | Input Hash Binding | `.agents/meta/10-gold-okf/invariants/` | T-003 (RFC 8785 SHA-256 hash) |
| **INV-007** | Tenant Isolation | `.agents/meta/10-gold-okf/invariants/` | T-001 (NATS subject namespace) |
| **INV-009** | JetStream Composite Keys | `.agents/rules/020_JETSTREAM_KEYS.md` | T-002 (Keys use `_`, never `:`) |
| **INV-012** | Schema Stability Snapshots | `.agents/meta/10-gold-okf/invariants/` | T-003, T-005 (Snapshot regression tests) |
| **INV-015** | Relative Paths in Documentation | `.agents/rules/030_RELATIVE_PATHS.md` | Doc compliance |
