# Tutorial: Integrate an Agent via REST / curl

This tutorial shows how to interact with a running Trust Gateway instance via its REST API using `curl`.

By the end, you'll understand how to:
1. Propose an action
2. Receive a policy decision
3. Obtain an ExecutionGrant JWT
4. Submit the grant for execution

---

## Prerequisites

- A running Trust Gateway instance with the REST API enabled on port `3060`
- `curl` and `jq` installed

> **Note:** This tutorial requires the full `gateway` daemon running with NATS. If you haven't set that up yet, start with the [standalone quickstart](../../README.md#-run-the-standalone-demo) to understand the concepts first.

---

## Step 1: Check Gateway Health

```bash
curl -s http://localhost:3060/health | jq .
```

Expected:
```json
{
  "status": "ok"
}
```

---

## Step 2: List Available Tools

```bash
curl -s http://localhost:3060/v1/tools/list | jq .
```

This returns the tools registered with the gateway and their policy constraints.

---

## Step 3: Propose a Read-Only Action

```bash
curl -s -X POST http://localhost:3060/v1/actions/propose \
  -H 'Content-Type: application/json' \
  -d '{
    "action_id": "demo-001",
    "tool_name": "inspect_schema",
    "arguments": {
      "dataset": "sales"
    },
    "operation_attributes": {
      "operation_kind": "read_only"
    }
  }' | jq .
```

Expected: The gateway evaluates the policy, determines this is a read-only operation, and **allows** it — returning an `ExecutionGrant` JWT.

Key fields in the response:
- `decision`: `"allowed"` — the policy approved the action
- `grant_jwt`: a signed JWT containing the `input_hash` of your exact arguments

---

## Step 4: Propose a Financial Mutation

```bash
curl -s -X POST http://localhost:3060/v1/actions/propose \
  -H 'Content-Type: application/json' \
  -d '{
    "action_id": "demo-002",
    "tool_name": "process_refund",
    "arguments": {
      "amount": "500.00",
      "reason": "customer request"
    },
    "operation_attributes": {
      "operation_kind": "financial_mutation",
      "amount_cents": 50000
    }
  }' | jq .
```

Expected: The policy sees `financial_mutation` with `amount_cents > 10000` and returns `"require_approval"` — a human must approve before a grant is minted.

---

## Step 5: Check Pending Approvals

```bash
curl -s http://localhost:3060/v1/approvals | jq .
```

This lists all actions awaiting human approval.

---

## What You've Learned

| Concept | What Happened |
|---|---|
| **Policy evaluation** | The gateway matched your `operation_kind` against rules in `policy.toml` |
| **Automatic allow** | Read-only operations were granted without human intervention |
| **Approval escalation** | Financial mutations above the threshold require human approval |
| **Cryptographic binding** | The grant JWT contains a SHA-256 hash of your exact arguments |

---

## Next Steps

- [Write a custom policy](../how-to/write-policy.md) — change the rules that govern decisions
- [Understand the protocol](../reference/PROTOCOL_SPEC.md) — learn the full specification
- [Security guarantees](../reference/security-guarantees.md) — understand the cryptographic properties
