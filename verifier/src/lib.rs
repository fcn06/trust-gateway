//! # Trust Gateway Verifier
//!
//! Independent library crate for validating short-lived Ed25519 **ExecutionGrant** tokens
//! and enforcing binding to canonicalized SHA-256 argument digests (`input_hash`).

pub mod canonical;
pub mod error;
pub mod grant;

pub use canonical::{canonical_hash, canonical_json};
pub use error::VerifierError;
pub use grant::{
    DelegationChain, ExecutionBudget, ExecutionGrant, FinancialImpact, GrantClearance, SignedGrant,
};

/// High-level function to verify input arguments against a grant's input_hash.
pub fn verify_input_binding(
    grant: &ExecutionGrant,
    expected_action: &str,
    arguments: &serde_json::Value,
) -> Result<(), VerifierError> {
    if !grant.allowed_action.is_empty() && grant.allowed_action != expected_action {
        return Err(VerifierError::ActionMismatch {
            allowed: grant.allowed_action.clone(),
            requested: expected_action.to_string(),
        });
    }

    let computed_hash = canonical_hash(arguments);
    if grant.input_hash != computed_hash {
        return Err(VerifierError::InputHashMismatch {
            claimed: grant.input_hash.clone(),
            computed: computed_hash,
        });
    }

    Ok(())
}
