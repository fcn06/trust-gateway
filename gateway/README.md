# Gateway

The central control plane daemon of Trust Gateway. It receives action proposals from AI agents, evaluates them against governance policies, and mints cryptographic execution grants.

## Purpose

The Gateway is the **policy authority** — the single point where "allow / deny / require approval" decisions are made. It exposes HTTP (REST + MCP) and NATS interfaces, evaluates the 4-layer policy engine, manages human-in-the-loop approvals, and issues Ed25519-signed `ExecutionGrant` JWTs.

## Inputs & Outputs

| Direction | What | Transport |
|---|---|---|
| **In** | `ProposedAction` from agents | REST (`POST /v1/actions/propose`), MCP (`tools/call`), NATS (`trust.v1.*.action.propose`) |
| **In** | Approval decisions from humans | REST (`POST /v1/approvals/:id/decision`) |
| **Out** | `ExecutionGrant` JWT | Returned to the proposing agent |
| **Out** | Audit events | NATS JetStream |
| **Out** | Tool listings | REST (`GET /v1/tools/list`), MCP (`tools/list`), NATS |

## Dependencies

- **Internal crates**: `trust_core`, `trust_policy`, `trust_auth`, `trust_verifier`, `identity_context`, `ssi_crypto`
- **External**: `axum` (HTTP), `async-nats` (messaging), `jwt-simple` (JWT), `rmcp` (MCP protocol)
- **Runtime**: NATS server with JetStream (for production mode)

## How to Run

```bash
# Development (requires NATS running on localhost:4222)
cargo run -p gateway

# With custom policy
POLICY_PATH=config/my-policy.toml cargo run -p gateway
```

The gateway listens on port `3060` by default.

## How to Test

```bash
# Unit tests
cargo test -p gateway --lib

# Integration tests
cargo test -p gateway
```

## Public API

### REST Endpoints

| Endpoint | Method | Description |
|---|---|---|
| `/health` | GET | Health check |
| `/v1/actions/propose` | POST | Propose an action for policy evaluation |
| `/v1/tools/list` | GET | List available governed tools |
| `/v1/approvals` | GET | List pending human approvals |
| `/v1/approvals/:id/decision` | POST | Submit approval/denial |
| `/v1/mcp/sse` | GET | MCP Server-Sent Events transport |
| `/v1/mcp/messages` | POST | MCP Streamable HTTP transport |

### Key Source Files

| File | Responsibility |
|---|---|
| `main.rs` | Binary entry point and server startup |
| `router.rs` | HTTP route configuration |
| `gateway.rs` | Core gateway logic |
| `grant.rs` | ExecutionGrant minting |
| `approval_daemon.rs` | Human approval workflow |
| `mcp_sse.rs` | MCP protocol handler |
| `policy_api.rs` | Policy evaluation API |
| `audit_sink.rs` | Audit event publisher |

## Security Assumptions

- The gateway is the **sole authority** for minting `ExecutionGrant` JWTs
- All grant JWTs are signed with Ed25519 private keys held only by the gateway
- The gateway runs on the **private sovereign core** (Server 2), never on the public edge
- NATS connections are authenticated via nkey
