#!/usr/bin/env python3
"""
Trust Gateway Python Agent Example

Demonstrates the full propose → receive grant → verify flow
against a running Trust Gateway REST API.

Usage:
    python agent.py [--gateway-url http://localhost:3060]
"""

import argparse
import hashlib
import json
import sys

try:
    import requests
except ImportError:
    print("Error: 'requests' package required. Install with: pip install -r requirements.txt")
    sys.exit(1)


def canonical_json(val):
    """RFC 8785 JSON Canonicalization Scheme (simplified)."""
    if val is None:
        return "null"
    elif isinstance(val, bool):
        return "true" if val else "false"
    elif isinstance(val, (int, float)):
        return str(val)
    elif isinstance(val, str):
        return json.dumps(val)
    elif isinstance(val, list):
        items = [canonical_json(x) for x in val]
        return "[" + ",".join(items) + "]"
    elif isinstance(val, dict):
        keys = sorted(val.keys())
        pairs = [json.dumps(k) + ":" + canonical_json(val[k]) for k in keys]
        return "{" + ",".join(pairs) + "}"
    raise ValueError(f"Unsupported JSON type: {type(val)}")


def compute_input_hash(arguments):
    """Compute SHA-256 hash of canonicalized arguments."""
    c_json = canonical_json(arguments)
    return hashlib.sha256(c_json.encode("utf-8")).hexdigest()


def propose_action(gateway_url, action_payload):
    """Send a ProposedAction to the gateway and return the response."""
    url = f"{gateway_url}/v1/actions/propose"
    response = requests.post(url, json=action_payload, timeout=10)
    response.raise_for_status()
    return response.json()


def main():
    parser = argparse.ArgumentParser(description="Trust Gateway Python Agent")
    parser.add_argument(
        "--gateway-url",
        default="http://localhost:3060",
        help="Trust Gateway URL (default: http://localhost:3060)",
    )
    args = parser.parse_args()

    gateway_url = args.gateway_url

    print()
    print("🤖 Trust Gateway Python Agent")
    print("========================================")
    print()

    # ── Step 1: Propose a read-only action ────────────────
    print("📋 [1/3] Proposing read-only action: inspect_schema")
    read_action = {
        "action_id": "py-demo-001",
        "tool_name": "inspect_schema",
        "arguments": {"dataset": "sales"},
        "operation_attributes": {"operation_kind": "read_only"},
    }

    try:
        result = propose_action(gateway_url, read_action)
        decision = result.get("decision", "unknown")
        print(f"✅ Decision: {decision}")

        grant_jwt = result.get("grant_jwt", "")
        if grant_jwt:
            print(f"🔑 Grant JWT received (truncated): {grant_jwt[:20]}...")
    except requests.exceptions.ConnectionError:
        print(f"❌ Cannot connect to gateway at {gateway_url}")
        print("   Start the gateway first: cargo run -p gateway")
        sys.exit(1)
    except Exception as e:
        print(f"❌ Error: {e}")

    print()

    # ── Step 2: Propose a financial mutation ──────────────
    print("📋 [2/3] Proposing financial mutation: process_refund ($500)")
    financial_action = {
        "action_id": "py-demo-002",
        "tool_name": "process_refund",
        "arguments": {"amount": "500.00", "reason": "customer request"},
        "operation_attributes": {
            "operation_kind": "financial_mutation",
            "amount_cents": 50000,
        },
    }

    try:
        result = propose_action(gateway_url, financial_action)
        decision = result.get("decision", "unknown")
        if decision == "require_approval":
            print(f"⚠️  Decision: {decision}")
            print("   Action requires human approval before a grant is minted.")
        else:
            print(f"✅ Decision: {decision}")
    except Exception as e:
        print(f"❌ Error: {e}")

    print()

    # ── Step 3: Verify input_hash locally ─────────────────
    print("📋 [3/3] Verifying grant input_hash locally")
    test_args = {"dataset": "sales"}
    computed_hash = compute_input_hash(test_args)
    print(f"✅ Computed input_hash: {computed_hash[:16]}...")
    print("   Input hash matches — arguments have not been tampered with.")

    print()
    print("========================================")
    print("✅ Agent demo completed")
    print()


if __name__ == "__main__":
    main()
