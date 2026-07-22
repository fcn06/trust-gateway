# Quickstart Guide

## Build and Test Workspace

```bash
cd trust-gateway

# Build workspace release binaries
cargo build --workspace --release

# Run unit tests across all domain crates and adapters
cargo test --workspace --lib

# Run zero-dependency standalone control flow example
cargo run -p quickstart-standalone

# Run execution protocol conformance test vectors
cargo run -p conformance -- --vectors-dir test-vectors
```

## CLI Verification

```bash
cargo run -p trustctl -- audit verify test-vectors/valid_grant.json
```
