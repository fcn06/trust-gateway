# 🛡️ Trust Gateway

[![Rust](https://img.shields.io/badge/Rust-1.88-orange?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue)](LICENSE)
[![NATS](https://img.shields.io/badge/NATS-JetStream-green?logo=nats.io)](https://nats.io)
[![MCP](https://img.shields.io/badge/MCP-SSE%20%2B%20Streamable-purple)](https://modelcontextprotocol.io)

> **An open execution authorization protocol and gateway for AI agents.**

AI agents should be able to propose actions without automatically receiving the authority to execute them.

**Trust Gateway** sits between AI agents and side-effecting tools (SaaS APIs, databases, native scripts). It evaluates each proposed action against deterministic policy rules and issues a short-lived, Ed25519-signed **ExecutionGrant** cryptographically bound to exactly one tool and one set of parameters. 

Executors independently verify that grant before performing any mutation.

> **"Agents propose. Gateway decides. Executors verify."**

```text
                               ┌─────────────────────────┐
                               │       AI AGENT          │
                               └────────────┬────────────┘
                                            │ 1. Propose tool + args
                                            ▼
                               ┌─────────────────────────┐
                               │      TRUST GATEWAY      │
                               │ - Identity & Policy     │
                               │ - Approval & Grant Mint │
                               └────────────┬────────────┘
                                            │ 2. Short-lived ExecutionGrant
                                            ▼
                               ┌─────────────────────────┐
                               │      EXECUTOR HOST      │
                               │ - Ed25519 & Hash Verify │
                               │ - Execute Mutation      │
                               └────────────┬────────────┘
                                            │ 3. Execute mutation
                                            ▼
                               ┌─────────────────────────┐
                               │   TARGET SAAS / API     │
                               └─────────────────────────┘
```

---

## 🚀 Quickstart

Choose your preferred way to test and evaluate Trust Gateway:

### Path 1: Fastest — Docker Demo (No Rust toolchain required)

We aimed to minimize onboarding friction and bandwidth overhead: the demo container is statically compiled against `musl` and stripped to achieve a lightweight image footprint (~5 MB download / ~18 MB uncompressed).

Build and run the standalone demonstration locally in Docker:

```bash
# 1. Build local container image (or run: make demo-docker)
docker build -t trust-gateway-demo -f deploy/Dockerfile.demo .
# Tip: Prefix with BUILDPROVENANCE=false for a completely silent BuildKit build

# 2. Run standard execution demo
docker run --rm trust-gateway-demo

# 3. Simulate parameter tampering post-approval (BLOCKED live by executor)
docker run --rm trust-gateway-demo --tamper

# 4. Simulate grant replay attack (BLOCKED live by single-use nonce)
docker run --rm trust-gateway-demo --replay
```

### Path 2: Source Build (Rust 1.88+)

Run the standalone quickstart demo directly from source:

```bash
# Prerequisites: Rust 1.88+, build-essential / xcode-select, libssl-dev
cargo run -p quickstart-standalone

# Test attack simulations:
cargo run -p quickstart-standalone -- --tamper
cargo run -p quickstart-standalone -- --replay
```

<details>
<summary><b>View Standalone Demo Execution Output</b></summary>

```text
=====================================================
🛡️ Trust Gateway Standalone Control Flow Quickstart
=====================================================
📥 1. Received ProposedAction: tool='mock_refund'
⚖️ 2. Policy Decision: approved=true, reason='Action permitted under default policy'
🔑 3. Issued ExecutionGrant: id='grant_action-demo-001', input_hash='38c23c59...'
⚡ 4. Execution Result: status=Succeeded, duration=5ms
🔒 5. Sanitized Output:
{
  "account_email": "[REDACTED]",
  "amount": "50.00",
  "status": "refund_processed"
}
=====================================================
✅ Standalone execution completed successfully!
=====================================================
```
</details>

### Path 3: Full Platform Mode (Docker Compose)

Spin up the complete distributed topology (NATS JetStream + Gateway + Executor Host):

```bash
docker compose -f deploy/docker-compose.yml up -d
```

---

## 🔌 Quick Integration Example

How does an AI agent propose an action to Trust Gateway over HTTP?

> **Prerequisite**: Start the Trust Gateway server first using **Path 3 (Docker Compose)**: `docker compose -f deploy/docker-compose.yml up -d` (or run `cargo run -p gateway`). This exposes the HTTP API on port `3060`.
>
> **Note on Authentication Scope**: This HTTP example demonstrates the core proposal control flow. In a production environment, the Gateway verifies individual user JWTs signed by an OIDC Relying Party (RP) / Identity Provider, extracts the `user_id` claims, and enforces user-specific policy rules and multi-tenant isolation (`tenant_id`).

Agents send a standardized REST payload to propose an action instead of invoking target APIs directly:

### 1. Agent Proposes Action

```bash
# Generate a valid development session JWT (HMAC-HS256 using dev secret):
HEADER=$(echo -n '{"alg":"HS256","typ":"JWT"}' | base64 -w0 | tr -d '=' | tr '/+' '_-')
PAYLOAD=$(echo -n '{"sub":"agent-001","exp":1999999999,"tenant_id":"tenant_demo"}' | base64 -w0 | tr -d '=' | tr '/+' '_-')
SECRET="dev-secret-do-not-use-in-production-1234567890"
SIGNATURE=$(echo -n "${HEADER}.${PAYLOAD}" | openssl dgst -sha256 -hmac "${SECRET}" -binary | base64 -w0 | tr -d '=' | tr '/+' '_-')
export SESSION_TOKEN="${HEADER}.${PAYLOAD}.${SIGNATURE}"

curl -X POST http://localhost:3060/v1/actions/propose \
  -H "Authorization: Bearer $SESSION_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "action_name": "claw_hello_world",
    "arguments": {
      "message": "Hello from HackerNews"
    }
  }'
```

### 2. Gateway Execution Response

The Gateway evaluates policy, mints an Ed25519 `ExecutionGrant`, dispatches execution to `executor-host` over NATS, scrubs the result, and returns the execution outcome:

```json
{
  "action_id": "7a636adf-bc8a-4305-a1a3-7c772bc8b27a",
  "status": "succeeded",
  "result": [
    {
      "text": "{\n  \"action_id\": \"7a636adf-bc8a-4305-a1a3-7c772bc8b27a\",\n  \"result\": \"Hello from HackerNews\",\n  \"skill\": \"claw_hello_world\"\n}",
      "type": "text"
    }
  ]
}
```

---

## 📖 Choose Your Path

| Goal | Resource / Guide | Time |
| :--- | :--- | :--- |
| **Integrate via REST** | [`docs/tutorials/rest-curl-agent.md`](docs/tutorials/rest-curl-agent.md) | 10 min |
| **Integrate via Python** | [`examples/python-agent/`](examples/python-agent/) | 10 min |
| **Connect an MCP client** | [`docs/tutorials/mcp-client.md`](docs/tutorials/mcp-client.md) | 15 min |
| **Write a custom policy** | [`docs/how-to/write-policy.md`](docs/how-to/write-policy.md) | 10 min |
| **Contribute to Rust workspace** | [`docs/getting-started.md`](docs/getting-started.md) | 15 min |
| **Understand limitations** | [`docs/concepts/LIMITATIONS.md`](docs/concepts/LIMITATIONS.md) | 5 min |

---

## 🌟 Core Architecture & Execution Modes

### 1. Managed Dispatch (Production Default)
The Gateway dispatches approved actions directly to isolated Executor Hosts over NATS JetStream channels (`exec.v1.<tenant>.<profile>.invoke`). The AI agent receives only the final, sanitized result and never handles execution grants or credentials.

### 2. Portable Grant Mode (REST & MCP Clients)
For external REST or MCP integrations, the client receives the short-lived `ExecutionGrant` JWT and presents it to the executor. 

> **Security Note on Portable Grants**:
> - Grants contain **zero SaaS credentials**.
> - Grants are cryptographically bound to exact parameters via SHA-256 `input_hash`.
> - Grants expire rapidly (e.g. 30-second TTL) and are single-use.
> - Executor hosts remain isolated and protected by network access rules.

```mermaid
sequenceDiagram
    autonumber
    actor Agent as 🤖 AI Agent
    participant Gateway as 🛡️ Trust Gateway
    participant Policy as ⚖️ Policy Engine
    participant Executor as ⚡ Executor Host
    participant API as 🌐 Target API

    Agent->>Gateway: Propose Action (tool, args)
    Gateway->>Policy: Evaluate policy.toml
    Policy-->>Gateway: Approved
    Gateway->>Gateway: Mint Ed25519 ExecutionGrant JWT
    alt Managed Dispatch (Production Default)
        Gateway->>Executor: Dispatch GrantedAction via NATS
    else Portable Grant Mode (REST / MCP)
        Gateway-->>Agent: Return ExecutionGrant JWT
        Agent->>Executor: Present Grant + Args
    end
    Executor->>Executor: Verify Ed25519 & SHA-256(input_hash)
    Executor->>API: Execute Mutation
    API-->>Executor: Raw Output
    Executor->>Gateway: Raw Result -> Gateway Egress Filter (trust-egress)
    Gateway-->>Agent: Return Sanitized ExecutionResult
```

---

## 🔒 Required Deployment Invariants

For Trust Gateway's security guarantees to hold in production, deployments **must** enforce these 5 physical invariants:

1. **Agents Hold No Credentials**: AI agents do not possess standing SaaS credentials, API keys, or database connection strings.
2. **Executors Are Sole Mutation Boundary**: Executor hosts are the only components network-authorized to mutate target systems.
3. **Agent-to-SaaS Paths Are Blocked**: Direct network paths between agent runtime nodes and SaaS endpoints are blocked by firewall or VPC rules.
4. **Mandatory Grant Verification**: Executor hosts refuse any execution attempt that lacks a valid, signed, single-use `ExecutionGrant`.
5. **Gateway-Only Grant Keys**: Ed25519 grant signing keys reside exclusively within the Gateway control plane.

### 🚫 Blocked Direct Path vs Authorized Path

```text
❌ FORBIDDEN DIRECT PATH (Blocked by VPC / Egress Filters):
   AI Agent ═════════════════════════════════════════════> Target SaaS API (BLOCKED)

✅ AUTHORIZED GOVERNED PATH:
   AI Agent ──► Trust Gateway ──► ExecutionGrant ──► Executor Host ──► Target SaaS API
```

---

## 🛡️ Threat Mitigation Matrix

| Vulnerability or Threat | Control Mechanism |
| :--- | :--- |
| **Prompt injection influences agent intent** | The proposed action still receives no execution authority unless it satisfies independent policy, identity, approval, and grant verification controls. |
| **Arguments tampered after approval** | SHA-256 `input_hash` verification fails at the executor boundary. |
| **Grant replayed by malicious actor** | Single-use nonce (`jti`) consumption at the executor rejects replay attempts. |
| **Session JWT used as execution grant** | JWT class separation strictly rejects non-grant token types. |
| **Agent runtime node compromised** | Agent holds no standing SaaS credentials. In managed mode, grants are dispatched directly to executors; in portable mode, grants remain short-lived, single-use, and bound to one parameter hash. |
| **Unknown or unregistered tool invocation** | Fail-closed default policy denies proposal immediately. |
| **Executor receives forged grant** | Ed25519 public key signature verification fails. |
| **Crash during execution** | `action_id`, provider-side idempotency keys, and state reconciliation reduce duplicate side-effects. Connectors without provider idempotency may require recovery handling. |

### Security Conformance

CI continuously verifies protocol security invariants via [`conformance/`](conformance/):
- ✓ Rejects modified parameters (`input_hash` mismatch)
- ✓ Rejects expired grants
- ✓ Rejects replayed grants (nonce consumed)
- ✓ Rejects wrong tool binding
- ✓ Rejects session JWT as execution grant
- ✓ Rejects HMAC grants in production mode (`LIANXI_ENV=production`)

---

## 🚫 What Trust Gateway Is Not

- **Not an LLM prompt firewall**: It operates at the deterministic execution boundary, not on conversation text.
- **Not a judge of reasoning correctness**: It evaluates policy compliance, not whether an agent's multi-step plan is optimal.
- **Not a replacement for network isolation**: It assumes agents are network-isolated from direct SaaS endpoints.
- **Not a blanket safe container for native scripts**: Arbitrary script execution requires host-level sandboxing (e.g., Wasmtime or Docker).

→ Read full system boundaries: [`docs/concepts/LIMITATIONS.md`](docs/concepts/LIMITATIONS.md)

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
name = "Require human approval for financial operations"
operation_kind = "financial_mutation"
action = "require_approval"
min_amount_cents = 10000
priority = 20
```

---

## 📚 Documentation & References

- **[Tutorials](docs/tutorials/)**: [REST / cURL Agent Guide](docs/tutorials/rest-curl-agent.md) | [MCP Client Guide](docs/tutorials/mcp-client.md)
- **[How-To Guides](docs/how-to/)**: [Authoring Policies](docs/how-to/write-policy.md) | [Adding Executor Tools](docs/how-to/add-tool.md) | [Human Approvals](docs/how-to/require-approval.md)
- **[Architecture & Concepts](docs/concepts/)**: [Visual Guide](docs/concepts/VISUAL_GUIDE.md) | [Architecture Breakdown](docs/concepts/ARCHITECTURE.md) | [Deployment Invariants](docs/concepts/DEPLOYMENT_TOPOLOGY.md) | [Limitations](docs/concepts/LIMITATIONS.md)
- **[Specifications & Security](docs/reference/)**: [Protocol Spec](docs/reference/PROTOCOL_SPEC.md) | [Security Guarantees](docs/reference/security-guarantees.md) | [Workspace Structure](docs/reference/WORKSPACE_STRUCTURE.md) | [Transports](docs/reference/API_TRANSPORTS.md)

---

## 🧰 Development

```bash
make doctor       # Verify prerequisites
make check        # Check workspace compilation
make test         # Run unit tests
make quickstart   # Run standalone demo
make demo-docker  # Build demo Docker image
make conformance  # Run protocol conformance vectors
make lint         # Run clippy & cargo fmt checks
```

---

Maintained by [lianxi.io](https://lianxi.io). Security disclosures: [`SECURITY.md`](SECURITY.md).