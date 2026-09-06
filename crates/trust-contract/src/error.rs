use crate::state_machine::ContractState;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ContractError {
    #[error("Contract '{0}' not found")]
    ContractNotFound(String),

    #[error("Contract '{0}' is not active (current state: {1})")]
    ContractNotActive(String, ContractState),

    #[error("Contract '{0}' has expired at {1}")]
    ContractExpired(String, String),

    #[error("Contract '{0}' has been revoked")]
    ContractRevoked(String),

    #[error("Contract hash mismatch: expected '{expected}', found '{actual}'")]
    ContractHashMismatch { expected: String, actual: String },

    #[error("Invalid state transition from '{from}' to '{to}'")]
    InvalidStateTransition {
        from: ContractState,
        to: ContractState,
    },

    #[error("Missing required attestation for DID '{0}'")]
    MissingAttestation(String),

    #[error("Invalid cryptographic attestation signature for signer '{0}'")]
    InvalidAttestation(String),

    #[error(
        "Counterparty mismatch: requester '{requester}' is not a party in contract '{contract_id}'"
    )]
    CounterpartyMismatch {
        requester: String,
        contract_id: String,
    },

    #[error("Capability '{0}' is not authorized in contract")]
    CapabilityNotAllowed(String),

    #[error("Operation '{operation}' on capability '{capability}' is not authorized")]
    OperationNotAllowed {
        capability: String,
        operation: String,
    },

    #[error("Argument constraint violation: {detail}")]
    ArgumentConstraintViolation { detail: String },

    #[error("Data policy violation: {detail}")]
    DataPolicyViolation { detail: String },

    #[error("Delegated authority insufficient: {0}")]
    DelegationInsufficient(String),

    #[error("Invalid delegated authority: {0}")]
    InvalidDelegation(String),

    #[error("Egress policy violation: {0}")]
    EgressPolicyViolation(String),

    #[error("Enterprise policy denied contract execution: {0}")]
    EnterprisePolicyDenied(String),

    #[error("Negotiation limit reached ({0})")]
    NegotiationLimitReached(String),

    #[error("Contract validation failed: {detail}")]
    InvalidContractStructure { detail: String },

    #[error("Serialization / canonicalization error: {0}")]
    SerializationError(String),

    #[error("Storage error: {0}")]
    StorageError(String),
}
