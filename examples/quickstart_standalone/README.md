# Standalone Quickstart

Zero-dependency standalone control flow demo. This is the **golden path** — the fastest way to see Trust Gateway in action.

## What It Demonstrates

This example runs the entire Trust Gateway control flow **in-process** without NATS, Docker, or any external dependencies:

1. An AI agent **proposes** a `mock_refund` action
2. The **policy engine** evaluates the action and approves it
3. A cryptographic **ExecutionGrant** JWT is minted with SHA-256 `input_hash` binding
4. The **reference executor** verifies the grant and executes the tool
5. **PII scrubbing** redacts sensitive data (email addresses) from the output

## Prerequisites

- Rust 1.88+ with cargo
- C compiler and OpenSSL dev headers (see [main README](../../README.md#prerequisites))

## Run

```bash
make quickstart
# or: cargo run -p quickstart-standalone
```

## Expected Output

```
=====================================================
🛡️ Trust Gateway Standalone Control Flow Quickstart
=====================================================
📥 1. Received ProposedAction: tool='mock_refund'
⚖️ 2. Policy Decision: approved=true, reason='Action permitted under default policy'
🔑 3. Issued ExecutionGrant: id='grant_action-demo-001', input_hash='38c23c59...'
⚡ 4. Execution Result: status=Succeeded, duration=5ms
🔒 5. Sanitized Output:
{
  "account_email": "[REDACTED]",
  "amount": "50.00",
  "status": "refund_processed"
}
=====================================================
✅ Standalone execution completed successfully!
=====================================================
```

## Key Concepts Demonstrated

| Output Line | Concept |
|---|---|
| `📥 1. Received ProposedAction` | Agents propose — they never execute directly |
| `⚖️ 2. Policy Decision: approved=true` | Gateway decides — policy engine evaluates rules |
| `🔑 3. Issued ExecutionGrant` | Cryptographic binding — grant includes `input_hash` |
| `⚡ 4. Execution Result` | Executors verify — grant is checked before execution |
| `🔒 5. Sanitized Output: [REDACTED]` | PII scrubbing — sensitive data is automatically removed |
