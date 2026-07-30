# Executor Host

The hardened execution runtime daemon that receives authorized actions, verifies their cryptographic grants, and dispatches tool execution in sandboxed profiles.

## Purpose

The Executor Host is the **execution authority** — it never trusts the caller. Every action must present a valid `ExecutionGrant` JWT. The host verifies the Ed25519 signature, validates the SHA-256 `input_hash` against the actual arguments, checks the single-use nonce, and only then executes the tool.

## Inputs & Outputs

| Direction | What | Transport |
|---|---|---|
| **In** | `GrantedAction` (grant JWT + arguments) | NATS (`trust.v1.*.action.invoke`) |
| **Out** | `ExecutionResult` (sanitized output) | NATS reply |

## Execution Profiles

| Profile | Description |
|---|---|
| `native-tool` | Runs sandboxed scripts from `native_tools/` |
| `connector` | Dispatches via OAuth2-authenticated HTTP calls |
| `vp` | Verifiable Presentation verification flow |

## Dependencies

- **Internal crates**: `trust-model`, `trust-executor-sdk`, `trust-grants`, `trust-egress`
- **External**: `async-nats` (messaging), `jwt-simple` (JWT verification)
- **Runtime**: NATS server with JetStream

## How to Run

```bash
# Requires NATS running on localhost:4222
EXECUTOR_PROFILE=native-tool cargo run -p executor_host
```

## How to Test

```bash
cargo test -p executor_host --lib
```

## Key Source Files

| File | Responsibility |
|---|---|
| `main.rs` | Binary entry point |
| `runtime.rs` | Execution runtime loop |
| `native_tools.rs` | Native tool execution profile |
| `connectors.rs` | OAuth2 connector profile |
| `vp.rs` | Verifiable Presentation profile |
| `jetstream_nonce_store.rs` | JetStream-backed single-use nonce tracking |
| `reconciliation.rs` | Crash recovery and incomplete execution reconciliation |

## Security Assumptions

- The executor host **never mints grants** — it only verifies them
- Grant verification checks: Ed25519 signature, `input_hash` SHA-256 binding, nonce uniqueness, expiration
- Nonces are stored in JetStream KV for distributed replay prevention
- PII scrubbing is applied to all outputs before returning results
