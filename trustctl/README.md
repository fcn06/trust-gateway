# `trustctl` — CLI Management Utility

A command-line tool for offline audit verification, policy linting, policy simulation, and grant inspection.

## Purpose

`trustctl` provides offline operational tooling for administrators and CI pipelines. It works without a running gateway or NATS server — all operations are performed against local files.

## Commands

| Command | Description |
|---|---|
| `trustctl audit verify <grant.json>` | Verify an ExecutionGrant's Ed25519 signature and input_hash binding |
| `trustctl policy lint <policy.toml>` | Validate a policy file's syntax and rule consistency |
| `trustctl policy simulate <policy.toml> <action.json>` | Simulate a policy decision without a running gateway |

## Dependencies

- **Internal**: `trust_verifier`, `policy_sdk`, `trust_ops`
- **External**: `serde_json`, `anyhow`

## How to Run

```bash
# Verify a grant
cargo run -p trustctl -- audit verify test-vectors/valid_grant.json

# Lint a policy
cargo run -p trustctl -- policy lint config/policy.standalone.toml
```

## How to Test

```bash
cargo test -p trustctl --lib
```

## Security Assumptions

- All verification is performed locally — no network calls
- Public keys must be provided or embedded in the test vector files
- This tool is intended for auditing and CI, not for production grant verification (use `trust_verifier` in-process for that)
