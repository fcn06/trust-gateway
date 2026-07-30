# Real-World Execution Flow Example

This document provides a step-by-step technical walkthrough of how an AI Agent prompt (`@agent Inspect the schema of the "sales" dataset.`) moves through the Trust Gateway zero-trust control plane when human approval is required.

---

## End-to-End Sequence Diagram

```
[User Chat Prompt: "@agent Inspect the schema of the 'sales' dataset."]
      │
      ▼
1. Agent Submits Intent ──► ProposedAction: tool='inspect_schema', args={"dataset": "sales"}
      │
      ▼
2. Policy Engine Evaluation ──► Policy rule matched: `require_approval` (High-Value Tool)
      │                     └─► Generated Approval ID: 05c6ef54-705e-4129-9df9-073f90e00d39
      ▼
3. Portal Human Approval ──► User clicks "Approve" in the Portal UI!
      │
      ▼
4. Grant Issuer ──────────► Minted Ed25519 `ExecutionGrant` JWT with SHA-256 `input_hash` binding
      │
      ▼
5. Sandboxed Executor Host ─► Verified Ed25519 signature + SHA-256 `input_hash` + single-use nonce
                            └─► Executed Python script (`run.py`) in sandboxed environment
                            └─► Egress Filter scrubbed PII
      │
      ▼
[Sanitized Output Returned] ──► { "columns": [ ... ], "dataset": "sales" }
```

---

## Detailed Step-by-Step Breakdown

### 1. Agent Intent Proposal (`ProposedAction`)
The agent receives the user prompt and constructs a standard `ProposedAction` struct (`crates/trust-model`):
- **Action**: `"inspect_schema"`
- **Arguments**: `{"dataset": "sales"}`
- **Agent DID**: `"did:example:agent-001"`
- **Tenant ID**: `"<tenant-uuid>"`

### 2. Policy Engine Evaluation (`crates/trust-policy`)
The Trust Gateway policy engine evaluates priority-ordered attribute rules against the proposal:
- Action `inspect_schema` matches a policy rule requiring human authorization (`require_approval`).
- The Gateway stores a pending approval state in NATS KV (`storage-nats-kv`) and returns `status: PendingApproval` along with a unique `approval_id` (`05c6ef54-705e-4129-9df9-073f90e00d39`).

### 3. Portal Human Approval
The user receives a notification in the UI:
- The user inspects the proposed tool (`inspect_schema`) and parameter arguments (`{"dataset": "sales"}`).
- Upon biometric or WebAuthn confirmation, the user clicks **Approve**.

### 4. Cryptographic Grant Issuance (`crates/trust-grants`)
Upon receiving the approval decision event:
- `crates/trust-canonical` computes the RFC 8785 canonical JSON SHA-256 `input_hash` of arguments `{"dataset": "sales"}`.
- `crates/trust-grants` issues a short-lived (60s) Ed25519-signed `ExecutionGrant` JWT containing the `input_hash`, target executor profile, and single-use `jti` replay nonce.

### 5. Sandboxed Execution & Egress Scrubbing (`executor_host` & `crates/trust-egress`)
The Executor Host receives the `ExecutionGrant` JWT:
- **Signature Verification**: Verifies the Ed25519 signature against the Gateway's public key (`verifier`).
- **Input Binding Verification**: Recalculates SHA-256 `input_hash` of actual arguments and asserts an exact match (preventing parameter tampering).
- **Single-Use Replay Check**: Verifies the `jti` nonce has not been executed previously.
- **Sandboxed Execution**: Runs `run.py` inside an isolated execution container.
- **PII Egress Filter**: Passes stdout through `crates/trust-egress` regex scrubbing to redact sensitive PII before returning the final sanitized response to the chat interface:
  ```json
  {
    "columns": [
      { "name": "id", "type": "integer" },
      { "name": "revenue", "type": "float" },
      { "name": "category", "type": "string" },
      { "name": "timestamp", "type": "timestamp" }
    ],
    "dataset": "sales"
  }
  ```
