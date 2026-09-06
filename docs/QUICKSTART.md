# Getting Started — Contributor Guide

This guide walks you through building, testing, and contributing to the Trust Gateway Rust workspace.

If you just want to **see what Trust Gateway does**, run the standalone demo first:

```bash
cargo run -p quickstart-standalone
```

---

## Prerequisites

### 1. Rust Toolchain (Required)

Install Rust 1.89+ via [rustup](https://www.rust-lang.org/tools/install):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

The repository includes a `rust-toolchain.toml` that pins the exact Rust version — `rustup` will automatically install the correct toolchain when you build.

### 2. System Build Dependencies (Required)

A C compiler and OpenSSL development headers are needed for native cryptography:

- **Linux (Ubuntu/Debian)**:
  ```bash
  sudo apt-get install -y build-essential pkg-config libssl-dev
  ```
- **Linux (Fedora/RHEL)**:
  ```bash
  sudo dnf install -y gcc openssl-devel pkg-config
  ```
- **macOS**:
  ```bash
  xcode-select --install
  ```

### 3. NATS Server (Optional)

Only needed if you want to run the full `gateway` or `executor_host` daemons (not required for the standalone demo or unit tests):

```bash
# See: https://docs.nats.io/running-a-nats-service/introduction/installation
nats-server -js
```

### Verify Your Environment

Run the doctor script to check all prerequisites:

```bash
make doctor
```

Expected output (all required checks should show ✅):

```
🩺 Trust Gateway — Environment Doctor
========================================

✅ Rust ................. 1.88.0
✅ Cargo ................ 1.88.0
✅ C compiler ........... found (cc)
✅ OpenSSL headers ...... 3.x.x
...
========================================
✅ All required checks passed. Ready to build!
```

---

## Build the Workspace

```bash
# Check compilation (fast — no codegen)
make check
# or: cargo check --workspace

# Build all crates in release mode
cargo build --workspace --release
```

Expected: compilation succeeds with no errors.

---

## Run Tests

### Unit Tests

```bash
make test
# or: cargo test --workspace --lib
```

Expected: all tests pass.

### Conformance Test Vectors

These verify the execution protocol's cryptographic guarantees against reference test vectors:

```bash
make conformance
# or: cargo run -p conformance -- --vectors-dir test-vectors
```

Expected: all vectors pass validation.

### CLI Audit Verification

```bash
make audit
# or: cargo run -p trustctl -- audit verify test-vectors/valid_grant.json
```

Expected: the CLI confirms the grant's Ed25519 signature and input hash are valid.

---

## Run the Standalone Demo

### 1. Happy Path Demo
```bash
cargo run -p quickstart-standalone
# or: make quickstart
```

### 2. Argument Tampering Attack Simulation (`--tamper`)
Demonstrates live rejection by the executor when an attacker tampers with action parameters after grant issuance:
```bash
cargo run -p quickstart-standalone -- --tamper
```

### 3. Grant Replay Attack Simulation (`--replay`)
Demonstrates live single-use grant/nonce rejection when an attacker attempts to re-submit an already consumed grant:
```bash
cargo run -p quickstart-standalone -- --replay
```

**What this demonstrates:**

| Mode / Step | What Happened |
|---|---|
| 📥 Step 1 | An AI agent proposed a `mock_refund` action with specific arguments |
| ⚖️ Step 2 | The policy engine evaluated the action against policy rules |
| 🔑 Step 3 | A cryptographic `ExecutionGrant` was minted, binding the exact arguments via SHA-256 hash |
| ⚡ Step 4 | The executor verified the grant signature and input hash, then executed |
| 🔒 Step 5 | PII (email addresses) was automatically redacted from the output |
| ⚠️ `--tamper` | Parameter tampering post-approval is rejected by executor due to `input_hash` mismatch |
| ⚠️ `--replay` | Re-submitting an already consumed grant is rejected by executor due to single-use nonce/JTI |

---

## Code Quality

```bash
# Check formatting
cargo fmt --all -- --check

# Run clippy lints
cargo clippy --workspace --all-targets -- -D warnings

# Or both at once:
make lint
```

---

## Troubleshooting

### `error: linker 'cc' not found`

Install a C compiler:
- Linux: `sudo apt-get install -y build-essential`
- macOS: `xcode-select --install`

### `failed to run custom build command for 'openssl-sys'`

Install OpenSSL development headers:
- Linux (Ubuntu/Debian): `sudo apt-get install -y pkg-config libssl-dev`
- Linux (Fedora/RHEL): `sudo dnf install -y openssl-devel pkg-config`
- macOS: `brew install openssl` (usually not needed — Xcode provides it)

### `error: rustc X.Y.Z is not supported` or unexpected compilation errors

The repository pins a specific Rust version in `rust-toolchain.toml`. Ensure `rustup` is up to date:
```bash
rustup update
```

### `Could not connect to NATS`

NATS is only required for the full `gateway` and `executor_host` daemons, not for the standalone quickstart or unit tests. If you need it:
```bash
nats-server -js
```

---

## Where to Go from Here

- **[`docs/concepts/VISUAL_GUIDE.md`](concepts/VISUAL_GUIDE.md)** — 5-minute visual architecture overview
- **[`docs/concepts/ARCHITECTURE.md`](concepts/ARCHITECTURE.md)** — Detailed architectural plane breakdown
- **[`docs/reference/PROTOCOL_SPEC.md`](reference/PROTOCOL_SPEC.md)** — Protocol specification
- **[`docs/reference/security-guarantees.md`](reference/security-guarantees.md)** — Security guarantees matrix
