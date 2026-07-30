#!/usr/bin/env bash
# scripts/doctor.sh — Environment health check for trust-gateway contributors
# Usage: bash scripts/doctor.sh
#
# Checks prerequisites for building and running trust-gateway.
# Exit code 0 = all required checks pass, 1 = one or more required checks fail.

set -euo pipefail

PASS="✅"
FAIL="❌"
WARN="⚠️ "
errors=0

echo ""
echo "🩺 Trust Gateway — Environment Doctor"
echo "========================================"
echo ""

# ── Required: Rust toolchain ──────────────────────────
if command -v rustc &>/dev/null; then
    rust_version=$(rustc --version | awk '{print $2}')
    # Check minimum version (1.75.0)
    major=$(echo "$rust_version" | cut -d. -f1)
    minor=$(echo "$rust_version" | cut -d. -f2)
    if [ "$major" -ge 1 ] && [ "$minor" -ge 75 ]; then
        echo "$PASS Rust ................. $rust_version"
    else
        echo "$FAIL Rust ................. $rust_version (need >= 1.75.0)"
        errors=$((errors + 1))
    fi
else
    echo "$FAIL Rust ................. not found"
    echo "   Install: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    errors=$((errors + 1))
fi

# ── Required: cargo ───────────────────────────────────
if command -v cargo &>/dev/null; then
    cargo_version=$(cargo --version | awk '{print $2}')
    echo "$PASS Cargo ................ $cargo_version"
else
    echo "$FAIL Cargo ................ not found"
    errors=$((errors + 1))
fi

# ── Required: C compiler ─────────────────────────────
if command -v cc &>/dev/null; then
    echo "$PASS C compiler ........... found (cc)"
elif command -v gcc &>/dev/null; then
    echo "$PASS C compiler ........... found (gcc)"
elif command -v clang &>/dev/null; then
    echo "$PASS C compiler ........... found (clang)"
else
    echo "$FAIL C compiler ........... not found"
    echo "   Linux: sudo apt-get install -y build-essential"
    echo "   macOS: xcode-select --install"
    errors=$((errors + 1))
fi

# ── Required: OpenSSL dev headers ─────────────────────
if command -v pkg-config &>/dev/null && pkg-config --exists openssl 2>/dev/null; then
    openssl_version=$(pkg-config --modversion openssl 2>/dev/null || echo "unknown")
    echo "$PASS OpenSSL headers ...... $openssl_version"
elif [ -f /usr/include/openssl/ssl.h ] || [ -f /usr/local/include/openssl/ssl.h ]; then
    echo "$PASS OpenSSL headers ...... found"
else
    echo "$FAIL OpenSSL headers ...... not found"
    echo "   Linux: sudo apt-get install -y pkg-config libssl-dev"
    echo "   macOS: brew install openssl (headers included with Xcode)"
    errors=$((errors + 1))
fi

echo ""
echo "── Optional (for full daemon mode) ──"
echo ""

# ── Optional: NATS server ─────────────────────────────
if command -v nats-server &>/dev/null; then
    nats_version=$(nats-server --version 2>/dev/null | head -1)
    echo "$PASS NATS server .......... $nats_version"
else
    echo "$WARN NATS server .......... not found (optional — needed for gateway/executor_host daemons)"
    echo "   Install: https://docs.nats.io/running-a-nats-service/introduction/installation"
fi

# ── Optional: Docker ──────────────────────────────────
if command -v docker &>/dev/null; then
    docker_version=$(docker --version 2>/dev/null | awk '{print $3}' | tr -d ',')
    echo "$PASS Docker ............... $docker_version"
else
    echo "$WARN Docker ............... not found (optional — needed for Docker demo path)"
fi

# ── Optional: Docker Compose ──────────────────────────
if docker compose version &>/dev/null 2>&1; then
    compose_version=$(docker compose version 2>/dev/null | awk '{print $NF}')
    echo "$PASS Docker Compose ....... $compose_version"
elif command -v docker-compose &>/dev/null; then
    compose_version=$(docker-compose --version 2>/dev/null | awk '{print $NF}')
    echo "$PASS Docker Compose ....... $compose_version (standalone)"
else
    echo "$WARN Docker Compose ....... not found (optional — needed for Docker demo path)"
fi

echo ""
echo "── Project checks ──"
echo ""

# ── Policy file ───────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

policy_file="$PROJECT_ROOT/config/policy.standalone.toml"
if [ -f "$policy_file" ]; then
    # Basic TOML syntax check: look for required sections
    if grep -q '\[governance\]' "$policy_file" && grep -q '\[\[rules\]\]' "$policy_file"; then
        echo "$PASS Policy file .......... valid ($policy_file)"
    else
        echo "$FAIL Policy file .......... found but missing required sections"
        errors=$((errors + 1))
    fi
else
    echo "$FAIL Policy file .......... not found at $policy_file"
    errors=$((errors + 1))
fi

# ── Port availability ─────────────────────────────────
check_port() {
    local port=$1
    local name=$2
    if command -v ss &>/dev/null; then
        if ss -tlnp 2>/dev/null | grep -q ":${port} "; then
            echo "$WARN Port $port ............ in use ($name)"
        else
            echo "$PASS Port $port ............ available ($name)"
        fi
    elif command -v lsof &>/dev/null; then
        if lsof -i ":${port}" &>/dev/null; then
            echo "$WARN Port $port ............ in use ($name)"
        else
            echo "$PASS Port $port ............ available ($name)"
        fi
    else
        echo "$WARN Port $port ............ cannot check ($name)"
    fi
}

check_port 3060 "Gateway HTTP"
check_port 4222 "NATS"

echo ""
echo "========================================"
if [ "$errors" -gt 0 ]; then
    echo "$FAIL $errors required check(s) failed. Fix the issues above before building."
    exit 1
else
    echo "$PASS All required checks passed. Ready to build!"
    echo ""
    echo "Next steps:"
    echo "  cargo run -p quickstart-standalone   # Run the standalone demo"
    echo "  make test                            # Run all tests"
    exit 0
fi
