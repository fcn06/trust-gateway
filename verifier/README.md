# Verifier (`trust_verifier`)

A lightweight, zero-trust execution grant verification library. Designed to be embedded in any service that needs to verify `ExecutionGrant` JWTs without depending on the full gateway.

## Purpose

The verifier is the **trust anchor** for executors and third-party integrators. It validates:

1. **Ed25519 signature** — proves the grant was minted by a trusted gateway
2. **SHA-256 `input_hash`** — proves the arguments haven't been tampered with since approval
3. **Expiration** — proves the grant is still valid
4. **Nonce uniqueness** — prevents replay attacks (when used with a nonce store)

## Inputs & Outputs

| Direction | What |
|---|---|
| **In** | Raw `ExecutionGrant` JWT string + original arguments |
| **Out** | `Ok(VerifiedGrant)` or `Err(VerificationError)` |

## Dependencies

- **External only**: `jwt-simple`, `sha2`, `serde`, `serde_json`, `schemars`
- **No internal workspace dependencies** — this crate is intentionally standalone

## How to Test

```bash
cargo test -p trust_verifier --lib
```

## Public API

```rust
// Core verification function
pub fn verify_grant(
    jwt: &str,
    public_key: &Ed25519PublicKey,
    arguments: &serde_json::Value,
) -> Result<VerifiedGrant, VerificationError>;
```

### Key Source Files

| File | Responsibility |
|---|---|
| `grant.rs` | Grant JWT verification and input_hash binding |
| `canonical.rs` | Deterministic JSON canonicalization for hash computation |
| `error.rs` | Typed verification errors |

## Security Assumptions

- This crate performs **signature verification only** — it does not mint grants
- The public key must be obtained through a trusted channel (out of band)
- Nonce tracking is the caller's responsibility (see `executor_host/jetstream_nonce_store.rs`)
