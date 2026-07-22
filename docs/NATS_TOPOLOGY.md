# NATS Leaf Node Topology & Routing Matrix

This document specifies subject-level permissions, routing direction, and leaf peering constraints between Server 1 (Public Edge) and Server 2 (Sovereign Core).

## Topology Principle

- **Server 1 (Public Domain)**: Ingress DIDComm, HTTP Webhooks, Edge Leaf Node. Zero private keys, zero sovereign databases.
- **Server 2 (Sovereign Core)**: Trust Gateway, Policy Engine, NATS JetStream Core, Executors, Agents.

## Routing Matrix

| Subject Pattern | Source Node | Destination Node | Traverses Server 1? | JetStream Persisted? | Implementation Adapter |
|---|---|---|---|---|---|
| `public.v1.ingress.>` | Public Gateway | Sovereign Host | Yes | No (Transit only) | `adapters/transport-nats` |
| `trust.v1.propose.>` | Agent / Host | Trust Gateway | No (Local S2) | Yes (`PROPOSALS` KV) | `adapters/storage-nats-kv` |
| `exec.v1.invoke.>` | Trust Gateway | Executor Host | No (Local S2) | No (Direct NATS) | `adapters/transport-nats` |
| `audit.action.>` | Trust Gateway / Executor | Audit Projector | No (Local S2) | Yes (`AUDIT_LOG` KV) | `crates/trust-audit` |
| `ui.v1.<tenant>.events` | Host BFF Relay | Browser Client | Optional (WebSocket) | Short-lived stream | NATS WebSocket Proxy |

## Security Invariants

1. Public Edge NATS credentials are explicitly DENIED access to `trust.v1.*`, `exec.v1.*`, `audit.*`, and all Sovereign KV stores.
2. Cross-server communication relies exclusively on NATS Leaf Node peering over encrypted port 7422 with NKey mutual authentication.
3. KV keys must strictly use `_` as separator (e.g. `grant_nonces`, `mcp_session_state`).
