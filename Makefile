# trust-gateway Makefile
# Convenience targets for building, testing, and running trust-gateway.

.PHONY: all check test quickstart conformance audit lint doctor

all: check test conformance

# ── Build ──────────────────────────────────────────────
check:
	@echo "🔍 Checking workspace compilation..."
	cargo check --workspace

# ── Test ───────────────────────────────────────────────
test:
	@echo "🧪 Running unit tests..."
	cargo test --workspace --lib

quickstart:
	@echo "🛡️  Running standalone quickstart demo..."
	cargo run -p quickstart-standalone

conformance:
	@echo "📋 Running conformance test vectors..."
	cargo run -p conformance -- --vectors-dir test-vectors

audit:
	@echo "🔐 Running CLI audit verification..."
	cargo run -p trustctl -- audit verify test-vectors/valid_grant.json

# ── Code Quality ───────────────────────────────────────
lint:
	@echo "🧹 Checking formatting..."
	cargo fmt --all -- --check
	@echo "📎 Running clippy..."
	cargo clippy --workspace --all-targets -- -D warnings

# ── Environment ────────────────────────────────────────
doctor:
	@echo "🩺 Running environment doctor..."
	@bash scripts/doctor.sh
