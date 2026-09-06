use crate::ceremony::execute_activation_ceremony;
use crate::error::ContractError;
use crate::model::{ContractAttestation, InteractionContract};
use crate::negotiation::{ContractProposal, NegotiationLimits};
use crate::state_machine::ContractState;
use crate::store::ContractStore;
use chrono::Utc;
use ed25519_dalek::VerifyingKey;
use std::collections::HashMap;

/// High-level orchestration service for contract negotiation, attestation, and lifecycle commands.
pub struct ContractNegotiationService<S: ContractStore> {
    store: S,
    limits: NegotiationLimits,
}

impl<S: ContractStore> ContractNegotiationService<S> {
    pub fn new(store: S) -> Self {
        Self {
            store,
            limits: NegotiationLimits::default(),
        }
    }

    pub fn with_limits(store: S, limits: NegotiationLimits) -> Self {
        Self { store, limits }
    }

    /// Submit a new contract proposal.
    pub async fn propose(
        &self,
        tenant_id: &str,
        proposal: ContractProposal,
    ) -> Result<ContractProposal, ContractError> {
        let mut contract = proposal.contract_draft.clone();
        contract.state = ContractState::Proposed;
        contract.updated_at = Utc::now();

        self.store.put_contract(tenant_id, contract).await?;
        Ok(proposal)
    }

    /// Submit a counterproposal linked to a previous proposal.
    pub async fn counter_propose(
        &self,
        tenant_id: &str,
        previous_proposal: &ContractProposal,
        counter_draft: InteractionContract,
    ) -> Result<ContractProposal, ContractError> {
        let counter_proposal = previous_proposal.create_counter(
            previous_proposal.recipient.clone(),
            previous_proposal.proposer.clone(),
            counter_draft,
            &self.limits,
        )?;

        self.store
            .put_contract(tenant_id, counter_proposal.contract_draft.clone())
            .await?;
        Ok(counter_proposal)
    }

    /// Accept a proposal, transitioning the contract to `Accepted`.
    pub async fn accept(
        &self,
        tenant_id: &str,
        contract_id: &str,
        party_did: &str,
    ) -> Result<InteractionContract, ContractError> {
        let mut contract = self
            .store
            .get_contract(tenant_id, contract_id)
            .await?
            .ok_or_else(|| ContractError::ContractNotFound(contract_id.to_string()))?;

        // Requester must be party to the contract
        if contract.issuer.did != party_did && contract.counterparty.did != party_did {
            return Err(ContractError::CounterpartyMismatch {
                requester: party_did.to_string(),
                contract_id: contract_id.to_string(),
            });
        }

        contract.state = contract.state.transition(ContractState::Accepted)?;
        contract.updated_at = Utc::now();

        self.store.put_contract(tenant_id, contract.clone()).await?;
        Ok(contract)
    }

    /// Reject a proposal, transitioning the contract to `Revoked`.
    pub async fn reject(
        &self,
        tenant_id: &str,
        contract_id: &str,
        party_did: &str,
    ) -> Result<InteractionContract, ContractError> {
        let mut contract = self
            .store
            .get_contract(tenant_id, contract_id)
            .await?
            .ok_or_else(|| ContractError::ContractNotFound(contract_id.to_string()))?;

        if contract.issuer.did != party_did && contract.counterparty.did != party_did {
            return Err(ContractError::CounterpartyMismatch {
                requester: party_did.to_string(),
                contract_id: contract_id.to_string(),
            });
        }

        contract.state = contract.state.transition(ContractState::Revoked)?;
        contract.updated_at = Utc::now();

        self.store.put_contract(tenant_id, contract.clone()).await?;
        Ok(contract)
    }

    /// Add an attestation signature to the contract.
    pub async fn add_attestation(
        &self,
        tenant_id: &str,
        contract_id: &str,
        attestation: ContractAttestation,
    ) -> Result<InteractionContract, ContractError> {
        let mut contract = self
            .store
            .get_contract(tenant_id, contract_id)
            .await?
            .ok_or_else(|| ContractError::ContractNotFound(contract_id.to_string()))?;

        // Remove any existing attestation by this signer
        contract
            .evidence
            .attestations
            .retain(|a| a.signer_did != attestation.signer_did);
        contract.evidence.attestations.push(attestation);

        // Check if both parties have attested
        let has_issuer = contract
            .evidence
            .attestations
            .iter()
            .any(|a| a.signer_did == contract.issuer.did);
        let has_counter = contract
            .evidence
            .attestations
            .iter()
            .any(|a| a.signer_did == contract.counterparty.did);

        if has_issuer && has_counter && contract.state == ContractState::Accepted {
            contract.state = ContractState::Attested;
        }

        contract.updated_at = Utc::now();
        self.store.put_contract(tenant_id, contract.clone()).await?;
        Ok(contract)
    }

    /// Run activation ceremony and transition contract to `Active`.
    pub async fn activate(
        &self,
        tenant_id: &str,
        contract_id: &str,
        party_keys: &HashMap<String, VerifyingKey>,
    ) -> Result<InteractionContract, ContractError> {
        let contract = self
            .store
            .get_contract(tenant_id, contract_id)
            .await?
            .ok_or_else(|| ContractError::ContractNotFound(contract_id.to_string()))?;

        let activated = execute_activation_ceremony(contract, party_keys)?;
        self.store
            .put_contract(tenant_id, activated.clone())
            .await?;
        Ok(activated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attestation::create_attestation;
    use crate::model::*;
    use crate::store::InMemoryContractStore;
    use ed25519_dalek::SigningKey;

    fn test_draft() -> InteractionContract {
        let now = Utc::now();
        InteractionContract {
            contract_id: "ctr_svc_01".to_string(),
            version: 1,
            state: ContractState::Draft,
            issuer: PartyIdentity::new_did("did:web:buyer.com"),
            counterparty: PartyIdentity::new_did("did:web:seller.com"),
            purpose: Purpose {
                code: "logistics".to_string(),
                description: "Logistics".to_string(),
            },
            capabilities: vec![ContractCapability {
                capability_id: "io.logistics.freight@v1".to_string(),
                operations: vec!["dispatch".to_string()],
                parameter_constraints: None,
                result_constraints: None,
            }],
            constraints: ContractConstraints::default(),
            data_policy: DataPolicy::default(),
            obligations: vec![],
            commercial_terms: None,
            validity: ContractValidity {
                valid_from: now - chrono::Duration::hours(1),
                valid_until: now + chrono::Duration::hours(48),
            },
            protocol: ProtocolBinding::default(),
            evidence: ContractEvidence::default(),
            parent_contract_id: None,
            previous_contract_hash: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn test_full_negotiation_to_activation_lifecycle() {
        let store = InMemoryContractStore::new();
        let service = ContractNegotiationService::new(store);
        let tenant_id = "tenant_alpha";

        let draft = test_draft();
        let proposal = ContractProposal::new_initial(
            "neg_001",
            draft.issuer.clone(),
            draft.counterparty.clone(),
            draft.clone(),
        )
        .unwrap();

        // 1. Propose
        let p1 = service.propose(tenant_id, proposal).await.unwrap();
        assert_eq!(p1.round, 1);

        // 2. Counter-propose
        let mut counter_draft = draft.clone();
        counter_draft.constraints.max_units = Some(500);
        let p2 = service
            .counter_propose(tenant_id, &p1, counter_draft)
            .await
            .unwrap();
        assert_eq!(p2.round, 2);

        // 3. Accept
        let accepted = service
            .accept(tenant_id, &draft.contract_id, "did:web:buyer.com")
            .await
            .unwrap();
        assert_eq!(accepted.state, ContractState::Accepted);

        // 4. Mutual Attestation
        let buyer_key = SigningKey::from_bytes(&[10u8; 32]);
        let seller_key = SigningKey::from_bytes(&[20u8; 32]);

        let att_buyer = create_attestation(&accepted, "did:web:buyer.com", &buyer_key).unwrap();
        let att_seller = create_attestation(&accepted, "did:web:seller.com", &seller_key).unwrap();

        service
            .add_attestation(tenant_id, &draft.contract_id, att_buyer)
            .await
            .unwrap();
        let attested = service
            .add_attestation(tenant_id, &draft.contract_id, att_seller)
            .await
            .unwrap();
        assert_eq!(attested.state, ContractState::Attested);

        // 5. Activate
        let mut keys = HashMap::new();
        keys.insert("did:web:buyer.com".to_string(), buyer_key.verifying_key());
        keys.insert("did:web:seller.com".to_string(), seller_key.verifying_key());

        let active = service
            .activate(tenant_id, &draft.contract_id, &keys)
            .await
            .unwrap();
        assert_eq!(active.state, ContractState::Active);
    }
}
