use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum VerifierError {
    #[error("Failed to parse token: {0}")]
    TokenParseError(String),

    #[error("Grant signature verification failed: {0}")]
    InvalidSignature(String),

    #[error("Grant has expired at {0}")]
    ExpiredGrant(i64),

    #[error("Input hash mismatch. Claimed: {claimed}, computed: {computed}")]
    InputHashMismatch { claimed: String, computed: String },

    #[error("Action mismatch. Allowed: {allowed}, requested: {requested}")]
    ActionMismatch { allowed: String, requested: String },

    #[error("Tenant ID mismatch. Expected: {expected}, got: {got}")]
    TenantMismatch { expected: String, got: String },

    #[error("Replay detected for JTI / Grant ID: {0}")]
    ReplayDetected(String),

    #[error("Attestation level insufficient: required {required}, got {got}")]
    AttestationInsufficient { required: String, got: String },

    #[error("Delegation depth exceeded: max {max}, got {current}")]
    DelegationDepthExceeded { max: u8, current: u8 },

    #[error("Execution budget exceeded: {0}")]
    BudgetExceeded(String),

    #[error("Canonicalization error: {0}")]
    CanonicalizationError(String),
}
