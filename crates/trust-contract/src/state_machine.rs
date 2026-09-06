use crate::error::ContractError;
use serde::{Deserialize, Serialize};

/// Deterministic contract lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContractState {
    Draft,
    Proposed,
    CounterProposed,
    Accepted,
    Attested,
    Active,
    Suspended,
    Revoked,
    Expired,
    Superseded,
}

impl std::fmt::Display for ContractState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Proposed => write!(f, "proposed"),
            Self::CounterProposed => write!(f, "counter_proposed"),
            Self::Accepted => write!(f, "accepted"),
            Self::Attested => write!(f, "attested"),
            Self::Active => write!(f, "active"),
            Self::Suspended => write!(f, "suspended"),
            Self::Revoked => write!(f, "revoked"),
            Self::Expired => write!(f, "expired"),
            Self::Superseded => write!(f, "superseded"),
        }
    }
}

impl ContractState {
    /// Determines whether a state transition from `self` to `target` is valid.
    pub fn can_transition_to(&self, target: ContractState) -> bool {
        matches!(
            (self, target),
            (Self::Draft, Self::Proposed)
                | (Self::Proposed, Self::CounterProposed)
                | (Self::Proposed, Self::Accepted)
                | (Self::Proposed, Self::Revoked)
                | (Self::CounterProposed, Self::CounterProposed)
                | (Self::CounterProposed, Self::Accepted)
                | (Self::CounterProposed, Self::Revoked)
                | (Self::Accepted, Self::Attested)
                | (Self::Accepted, Self::Revoked)
                | (Self::Attested, Self::Active)
                | (Self::Attested, Self::Revoked)
                | (Self::Active, Self::Suspended)
                | (Self::Active, Self::Revoked)
                | (Self::Active, Self::Expired)
                | (Self::Active, Self::Superseded)
                | (Self::Suspended, Self::Active)
                | (Self::Suspended, Self::Revoked)
                | (Self::Suspended, Self::Expired)
        )
    }

    /// Performs the state transition or returns a deterministic ContractError.
    pub fn transition(&self, target: ContractState) -> Result<ContractState, ContractError> {
        if self.can_transition_to(target) {
            Ok(target)
        } else {
            Err(ContractError::InvalidStateTransition {
                from: *self,
                to: target,
            })
        }
    }

    /// Returns true if the contract is in an active, enforceable state.
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Returns true if the contract is terminal (no further actions or transitions allowed).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Revoked | Self::Expired | Self::Superseded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_lifecycle() {
        let state = ContractState::Draft;
        let state = state.transition(ContractState::Proposed).unwrap();
        let state = state.transition(ContractState::CounterProposed).unwrap();
        let state = state.transition(ContractState::Accepted).unwrap();
        let state = state.transition(ContractState::Attested).unwrap();
        let state = state.transition(ContractState::Active).unwrap();
        assert!(state.is_active());
        let state = state.transition(ContractState::Superseded).unwrap();
        assert!(state.is_terminal());
    }

    #[test]
    fn test_invalid_transitions() {
        assert!(ContractState::Draft
            .transition(ContractState::Active)
            .is_err());
        assert!(ContractState::Expired
            .transition(ContractState::Active)
            .is_err());
        assert!(ContractState::Revoked
            .transition(ContractState::Active)
            .is_err());
    }
}
