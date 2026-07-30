// ─────────────────────────────────────────────────────────────
// Execution Grant types — re-exported from trust_verifier
//
// The narrow, action-specific JWT that replaces broad session
// tokens for connector execution.
// ─────────────────────────────────────────────────────────────

pub use trust_verifier::{
    DelegationChain, ExecutionBudget, ExecutionGrant, FinancialImpact, GrantClearance, SignedGrant,
};
