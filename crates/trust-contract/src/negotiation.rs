use crate::error::ContractError;
use crate::model::{InteractionContract, PartyIdentity};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Maximum bounds to prevent infinite agent negotiation loops.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiationLimits {
    pub max_rounds: u32,
    pub max_duration_secs: i64,
    pub max_contract_size_bytes: usize,
}

impl Default for NegotiationLimits {
    fn default() -> Self {
        Self {
            max_rounds: 8,
            max_duration_secs: 120,
            max_contract_size_bytes: 64 * 1024, // 64 KB
        }
    }
}

/// A single step in a structured negotiation session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ContractProposal {
    pub negotiation_id: String,
    pub round: u32,
    pub proposer: PartyIdentity,
    pub recipient: PartyIdentity,
    pub contract_draft: InteractionContract,
    pub proposal_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_proposal_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ContractProposal {
    /// Create the initial proposal in a negotiation.
    pub fn new_initial(
        negotiation_id: impl Into<String>,
        proposer: PartyIdentity,
        recipient: PartyIdentity,
        contract_draft: InteractionContract,
    ) -> Result<Self, ContractError> {
        let proposal_hash = crate::hash::compute_contract_hash(&contract_draft)?;
        Ok(Self {
            negotiation_id: negotiation_id.into(),
            round: 1,
            proposer,
            recipient,
            contract_draft,
            proposal_hash,
            previous_proposal_hash: None,
            created_at: Utc::now(),
        })
    }

    /// Create a counterproposal linked to this previous proposal.
    pub fn create_counter(
        &self,
        proposer: PartyIdentity,
        recipient: PartyIdentity,
        mut counter_draft: InteractionContract,
        limits: &NegotiationLimits,
    ) -> Result<Self, ContractError> {
        if self.round >= limits.max_rounds {
            return Err(ContractError::NegotiationLimitReached(format!(
                "Max rounds ({}) exceeded",
                limits.max_rounds
            )));
        }

        let serialized = serde_json::to_string(&counter_draft)
            .map_err(|e| ContractError::SerializationError(e.to_string()))?;
        if serialized.len() > limits.max_contract_size_bytes {
            return Err(ContractError::NegotiationLimitReached(format!(
                "Contract size ({} bytes) exceeds limit ({} bytes)",
                serialized.len(),
                limits.max_contract_size_bytes
            )));
        }

        counter_draft.state = crate::state_machine::ContractState::CounterProposed;
        counter_draft.updated_at = Utc::now();

        let proposal_hash = crate::hash::compute_contract_hash(&counter_draft)?;

        Ok(Self {
            negotiation_id: self.negotiation_id.clone(),
            round: self.round + 1,
            proposer,
            recipient,
            contract_draft: counter_draft,
            proposal_hash,
            previous_proposal_hash: Some(self.proposal_hash.clone()),
            created_at: Utc::now(),
        })
    }
}
