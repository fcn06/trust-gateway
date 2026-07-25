# Messaging Topology & Routing Principles

This document describes, at a conceptual level, how NATS is used to connect the public edge and the governance core, and the routing principles that keep them isolated.

## Topology Principle

- **Public edge**: handles inbound ingress (webhooks, HTTP), and relays a narrow set of transit-only messages toward the governance core. It holds zero private keys and zero durable state.
- **Governance core**: runs the Trust Gateway policy engine, JetStream-backed durable state, and executors.

## Routing Principles

- Messages that carry action proposals, execution grants, or audit events are scoped to the governance core and are never routable from the public edge.
- Only transit-only ingress subjects (no persistence, no replay) are permitted to cross from the public edge into the core.
- Real-time status/audit updates reach browser clients only through a proxy layer — internal messaging endpoints are never exposed directly to the public internet.
- Cross-boundary communication is mutually authenticated and encrypted in transit; state stores (JetStream KV buckets) are never synchronized across the boundary — only message passing is permitted.

## Security Invariants

1. Public-edge credentials are explicitly denied access to internal governance, execution, and audit subjects.
2. Cross-boundary communication relies exclusively on mutually authenticated, encrypted transport.
3. Internal KV bucket naming and exact subject strings are an operational deployment detail, intentionally not enumerated here.

Deployers should define their own subject-routing matrix and NATS ACLs as part of their own deployment (see `deploy/` for a reference starting point).
