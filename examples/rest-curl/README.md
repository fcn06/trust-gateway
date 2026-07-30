# REST / curl Integration Example

This example demonstrates how to interact with a running Trust Gateway instance via its REST API using `curl` and `bash`.

## Prerequisites

- A running Trust Gateway instance with the REST API enabled on port `3060`
- `curl` and `jq` installed

> **Note:** This example requires the full `gateway` daemon running with NATS. If you haven't set that up yet, start with the [standalone quickstart](../quickstart_standalone/) to understand the concepts first.

## Sample Payloads

### Read-Only Action (will be allowed)

```json
{
  "action_id": "rest-demo-001",
  "tool_name": "inspect_schema",
  "arguments": {
    "dataset": "sales"
  },
  "operation_attributes": {
    "operation_kind": "read_only"
  }
}
```

### Financial Mutation (will require approval)

```json
{
  "action_id": "rest-demo-002",
  "tool_name": "process_refund",
  "arguments": {
    "amount": "500.00",
    "reason": "customer request"
  },
  "operation_attributes": {
    "operation_kind": "financial_mutation",
    "amount_cents": 50000
  }
}
```

## Run

```bash
bash run.sh
```

Or execute individual requests manually — see the [REST tutorial](../../docs/tutorials/rest-curl-agent.md) for step-by-step instructions.
