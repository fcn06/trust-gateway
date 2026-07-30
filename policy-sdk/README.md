# Policy SDK (`policy_sdk`)

A 4-layer hierarchical policy engine for zero-trust authorization decisions.

## Purpose

The Policy SDK parses, validates, and evaluates governance policies defined in `policy.toml` files. It implements the attribute-based policy evaluation that the gateway uses to make allow/deny/require-approval decisions.

## Inputs & Outputs

| Direction | What |
|---|---|
| **In** | `policy.toml` file + `ProposedAction` |
| **Out** | `PolicyDecision` (allow, deny, require_approval, with reason) |

## Dependencies

- **Internal**: `trust_verifier`, `trust-model`
- **External**: `serde`, `toml`, `thiserror`

## How to Test

```bash
cargo test -p policy_sdk --lib
```

## Key Source Files

| File | Responsibility |
|---|---|
| `lib.rs` | Public API and re-exports |
| `evaluator.rs` | Rule matching and priority-ordered evaluation |
| `layers.rs` | 4-layer policy hierarchy |
| `simulation.rs` | Policy simulation for testing |

## Policy Layers

The 4-layer hierarchy (in evaluation order):

1. **System invariants** — hardcoded security rules that cannot be overridden
2. **Organizational policy** — tenant-wide governance rules
3. **Tool-specific rules** — per-tool overrides
4. **Default action** — fallback when no rule matches

## Public API

```rust
// Evaluate a proposed action against loaded policy
pub fn evaluate(action: &ProposedAction, policy: &Policy) -> PolicyDecision;

// Simulate a policy without side effects
pub fn simulate(action: &ProposedAction, policy: &Policy) -> SimulationResult;
```

## Security Assumptions

- Policy files are loaded from the local filesystem — the gateway trusts the policy file's integrity
- The `default_action = "deny"` pattern ensures fail-closed behavior
- Policy evaluation is deterministic — same inputs always produce the same decision
