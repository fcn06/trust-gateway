# Python Agent Example

A Python client that demonstrates the full propose → receive grant → verify flow against a running Trust Gateway REST API.

## What It Demonstrates

1. Proposing an action to the gateway via HTTP POST
2. Receiving a policy decision (allow / deny / require_approval)
3. Receiving an `ExecutionGrant` JWT for allowed actions
4. Verifying the grant's `input_hash` locally before execution

## Prerequisites

- Python 3.8+
- A running Trust Gateway instance on `localhost:3060`

```bash
pip install -r requirements.txt
```

## Run

```bash
python agent.py
```

Or with a custom gateway URL:

```bash
python agent.py --gateway-url http://your-gateway:3060
```

## Expected Output

```
🤖 Trust Gateway Python Agent
========================================

📋 [1/3] Proposing read-only action: inspect_schema
✅ Decision: allowed
🔑 Grant JWT received (truncated): eyJ0eXAi...

📋 [2/3] Proposing financial mutation: process_refund ($500)
⚠️  Decision: require_approval
   Action requires human approval before a grant is minted.

📋 [3/3] Verifying grant input_hash locally
✅ Input hash matches — arguments have not been tampered with.

========================================
✅ Agent demo completed
```

## Note

This is an **integration client example** demonstrating how external Python applications can interact with Trust Gateway. It is not part of the core Rust workspace.
