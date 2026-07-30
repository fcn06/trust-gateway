# Adapters — Technology Transports & Storage

Technology-specific adapters that implement the ports defined by the domain crates. Following hexagonal architecture, these adapters can be swapped without changing business logic.

## Adapter Index

| Adapter | Purpose |
|---|---|
| **[`transport-nats/`](transport-nats/)** | NATS pub/sub message router for action proposals, grant delivery, and audit streaming |
| **[`storage-nats-kv/`](storage-nats-kv/)** | NATS JetStream key-value state adapter for nonce tracking and approval state |

## Architecture

```
Domain Crates (trust-model, trust-grants, ...)
        │
        │ define ports (traits)
        ▼
    Adapters
        │
        │ implement ports using specific technology
        ▼
  NATS / JetStream
```

## How to Test

```bash
# Integration tests (require a running NATS server)
cargo test -p transport-nats
cargo test -p storage-nats-kv
```

## Security Assumptions

- NATS connections use nkey authentication
- Subject-level permissions enforce tenant isolation
- JetStream KV provides at-least-once delivery for audit events
