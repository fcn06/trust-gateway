#!/usr/bin/env bash
# examples/rest-curl/run.sh
# Demonstrates Trust Gateway REST API interaction using curl.
#
# Prerequisites:
#   - Trust Gateway running on localhost:3060
#   - curl and jq installed
#
# Usage: bash run.sh [gateway_url]

set -euo pipefail

GATEWAY_URL="${1:-http://localhost:3060}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo ""
echo "🛡️  Trust Gateway — REST API Demo"
echo "========================================"
echo "Gateway: $GATEWAY_URL"
echo ""

# ── Step 1: Health Check ──────────────────────────────
echo "📋 [1/4] Checking gateway health..."
health=$(curl -sf "${GATEWAY_URL}/health" 2>/dev/null) || {
    echo "❌ Gateway not reachable at ${GATEWAY_URL}/health"
    echo "   Start the gateway first: cargo run -p gateway"
    exit 1
}
echo "✅ Gateway is healthy"
echo ""

# ── Step 2: Propose Read-Only Action ──────────────────
echo "📋 [2/4] Proposing read-only action (inspect_schema)..."
read_response=$(curl -sf -X POST "${GATEWAY_URL}/v1/actions/propose" \
    -H 'Content-Type: application/json' \
    -d @"${SCRIPT_DIR}/propose-read.json" 2>/dev/null) || {
    echo "❌ Failed to propose read action"
    exit 1
}
echo "Response:"
echo "$read_response" | jq . 2>/dev/null || echo "$read_response"
echo ""

# ── Step 3: Propose Financial Mutation ────────────────
echo "📋 [3/4] Proposing financial mutation (process_refund, \$500)..."
financial_response=$(curl -sf -X POST "${GATEWAY_URL}/v1/actions/propose" \
    -H 'Content-Type: application/json' \
    -d @"${SCRIPT_DIR}/propose-financial.json" 2>/dev/null) || {
    echo "❌ Failed to propose financial action"
    exit 1
}
echo "Response:"
echo "$financial_response" | jq . 2>/dev/null || echo "$financial_response"
echo ""

# ── Step 4: Check Pending Approvals ───────────────────
echo "📋 [4/4] Checking pending approvals..."
approvals=$(curl -sf "${GATEWAY_URL}/v1/approvals" 2>/dev/null) || {
    echo "⚠️  Could not fetch approvals (endpoint may not be available)"
}
if [ -n "${approvals:-}" ]; then
    echo "Response:"
    echo "$approvals" | jq . 2>/dev/null || echo "$approvals"
fi
echo ""

echo "========================================"
echo "✅ REST API demo completed"
echo ""
echo "Next steps:"
echo "  - Modify config/policy.standalone.toml to change policy rules"
echo "  - See docs/how-to/write-policy.md for policy authoring guide"
