# How-To: Require Human Approval

> **Status:** This guide is a work in progress.

This guide will explain how to set up human-in-the-loop approval for sensitive operations.

---

## Overview

Trust Gateway supports escalating certain actions to human reviewers before granting execution authority. When a policy rule specifies `action = "require_approval"`, the gateway:

1. Holds the action in a pending approval queue
2. Exposes it via the Human Approval API (`GET /v1/approvals`)
3. Waits for a human to approve or deny (`POST /v1/approvals/:id/decision`)
4. Only mints an `ExecutionGrant` if the human approves

---

## Policy Configuration

```toml
[[rules]]
name = "Require approval for financial mutations"
operation_kind = "financial_mutation"
action = "require_approval"
min_amount_cents = 10000
priority = 20
```

---

## Approval API

| Endpoint | Method | Description |
|---|---|---|
| `/v1/approvals` | GET | List all pending approvals |
| `/v1/approvals/:id/decision` | POST | Submit an approval or denial |

### Approve an action

```bash
curl -X POST http://localhost:3060/v1/approvals/ACTION_ID/decision \
  -H 'Content-Type: application/json' \
  -d '{"approved": true, "reviewer": "admin@example.com"}'
```

### Deny an action

```bash
curl -X POST http://localhost:3060/v1/approvals/ACTION_ID/decision \
  -H 'Content-Type: application/json' \
  -d '{"approved": false, "reviewer": "admin@example.com", "reason": "Amount too high"}'
```

### Tier 2 WebAuthn Session Enforcement

For actions configured with `tier = "tier2_re_authenticate"`, the Trust Gateway enforces that the approver's session carries hardware-backed biometric verification (`AuthLevel::Level5WebAuthn` or `auth_method: "webauthn"`). 

If an approval request for a Tier 2 action is submitted with a standard session (`auth_level < 5`), the Gateway rejects the request with `403 Forbidden`. Successful WebAuthn approvals record `resolution_method = "webauthn_session"` in the audit record.

---

## Integration

<!-- TODO: Building an approval dashboard -->
<!-- TODO: Webhook notifications for pending approvals -->
<!-- TODO: Timeout and auto-deny configuration -->

*This guide is under development. See the [REST/curl tutorial](../tutorials/rest-curl-agent.md) for a working example of the approval flow.*

---

## Next Steps

- [Write a policy](write-policy.md) — configure which operations require approval
- [Security guarantees](../reference/security-guarantees.md) — understand the cryptographic properties of the approval flow
