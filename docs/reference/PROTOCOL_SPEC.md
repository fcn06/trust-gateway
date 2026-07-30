# Open Execution Authorization Protocol Specification

Version: 1.0.0

## Domain Model Contracts (`crates/trust-model`)
- `ProposedAction`: Intent envelope submitted by agents.
- `ExecutionGrant`: Short-lived Ed25519-signed authorization grant.
- `GrantedAction`: Action payload dispatched to executors.
- `ExecutionResult`: Final outcome report containing `TransactionOutcomeState`.

## Canonical JSON & Hash Calculation (`crates/trust-canonical`)
Inputs are canonicalized per RFC 8785 (sorted keys, no whitespace) and hashed using SHA-256 (`input_hash`).

## Schema Contracts
Refer to standard JSON Schemas in [`protocol/`](../../protocol/):
- `execution-grant.schema.json`
- `proposal.schema.json`
- `result.schema.json`
