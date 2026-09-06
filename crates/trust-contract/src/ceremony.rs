use crate::attestation::verify_attestation;
use crate::error::ContractError;
use crate::hash::verify_contract_hash;
use crate::model::InteractionContract;
use crate::state_machine::ContractState;
use crate::validation::validate_contract_structure;
use chrono::Utc;
use ed25519_dalek::VerifyingKey;
use std::collections::HashMap;

/// Performs the deterministic 9-step contract activation ceremony (spec §18).
/// Transforms an `Accepted` / `Attested` contract into an enforceable `Active` contract.
pub fn execute_activation_ceremony(
    mut contract: InteractionContract,
    party_keys: &HashMap<String, VerifyingKey>,
) -> Result<InteractionContract, ContractError> {
    // 1. Structural validity check
    validate_contract_structure(&contract)?;

    // 2. State precondition check (must be Accepted or Attested)
    if contract.state != ContractState::Accepted && contract.state != ContractState::Attested {
        return Err(ContractError::InvalidStateTransition {
            from: contract.state,
            to: ContractState::Active,
        });
    }

    // 3. Hash computation & verification
    let computed_hash = crate::hash::compute_contract_hash(&contract)?;
    verify_contract_hash(&contract, &computed_hash)?;

    // 4. Temporal validity check (must not be expired)
    let now = Utc::now();
    if now > contract.validity.valid_until {
        return Err(ContractError::ContractExpired(
            contract.contract_id.clone(),
            contract.validity.valid_until.to_rfc3339(),
        ));
    }

    // 5. Attestation presence for required parties (both issuer and counterparty)
    let has_issuer_attestation = contract
        .evidence
        .attestations
        .iter()
        .any(|a| a.signer_did == contract.issuer.did);

    if !has_issuer_attestation {
        return Err(ContractError::MissingAttestation(
            contract.issuer.did.clone(),
        ));
    }

    let has_counterparty_attestation = contract
        .evidence
        .attestations
        .iter()
        .any(|a| a.signer_did == contract.counterparty.did);

    if !has_counterparty_attestation {
        return Err(ContractError::MissingAttestation(
            contract.counterparty.did.clone(),
        ));
    }

    // 6. Cryptographic signature verification for each attestation
    for attestation in &contract.evidence.attestations {
        let key = party_keys.get(&attestation.signer_did).ok_or_else(|| {
            ContractError::InvalidAttestation(format!(
                "Public key not available for signer '{}'",
                attestation.signer_did
            ))
        })?;

        verify_attestation(&contract, attestation, key)?;
    }

    // 7. Transition state to Active
    contract.state = ContractState::Active;
    contract.updated_at = Utc::now();

    Ok(contract)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attestation::create_attestation;
    use crate::model::*;
    use ed25519_dalek::SigningKey;

    #[test]
    fn test_successful_activation_ceremony() {
        let issuer_key = SigningKey::from_bytes(&[1u8; 32]);
        let counterparty_key = SigningKey::from_bytes(&[2u8; 32]);

        let issuer_did = "did:web:issuer.example.com";
        let counterparty_did = "did:web:counterparty.example.com";

        let mut party_keys = HashMap::new();
        party_keys.insert(issuer_did.to_string(), issuer_key.verifying_key());
        party_keys.insert(
            counterparty_did.to_string(),
            counterparty_key.verifying_key(),
        );

        let now = Utc::now();
        let mut contract = InteractionContract {
            contract_id: "ctr_ceremony_01".to_string(),
            version: 1,
            state: ContractState::Accepted,
            issuer: PartyIdentity::new_did(issuer_did),
            counterparty: PartyIdentity::new_did(counterparty_did),
            purpose: Purpose {
                code: "order_fulfillment".to_string(),
                description: "Order Fulfillment".to_string(),
            },
            capabilities: vec![ContractCapability {
                capability_id: "io.example.orders@v1".to_string(),
                operations: vec!["fulfill".to_string()],
                parameter_constraints: None,
                result_constraints: None,
            }],
            constraints: ContractConstraints::default(),
            data_policy: DataPolicy::default(),
            obligations: vec![],
            commercial_terms: None,
            validity: ContractValidity {
                valid_from: now - chrono::Duration::hours(1),
                valid_until: now + chrono::Duration::hours(24),
            },
            protocol: ProtocolBinding::default(),
            evidence: ContractEvidence::default(),
            parent_contract_id: None,
            previous_contract_hash: None,
            created_at: now,
            updated_at: now,
        };

        // Attach mutual attestations
        let att1 = create_attestation(&contract, issuer_did, &issuer_key).unwrap();
        let att2 = create_attestation(&contract, counterparty_did, &counterparty_key).unwrap();
        contract.evidence.attestations = vec![att1, att2];

        // Execute ceremony
        let activated = execute_activation_ceremony(contract, &party_keys).unwrap();
        assert_eq!(activated.state, ContractState::Active);
    }

    #[test]
    fn test_ceremony_fails_if_counterparty_missing() {
        let issuer_key = SigningKey::from_bytes(&[1u8; 32]);
        let issuer_did = "did:web:issuer.example.com";
        let counterparty_did = "did:web:counterparty.example.com";

        let mut party_keys = HashMap::new();
        party_keys.insert(issuer_did.to_string(), issuer_key.verifying_key());

        let now = Utc::now();
        let mut contract = InteractionContract {
            contract_id: "ctr_ceremony_02".to_string(),
            version: 1,
            state: ContractState::Accepted,
            issuer: PartyIdentity::new_did(issuer_did),
            counterparty: PartyIdentity::new_did(counterparty_did),
            purpose: Purpose {
                code: "order_fulfillment".to_string(),
                description: "Order Fulfillment".to_string(),
            },
            capabilities: vec![ContractCapability {
                capability_id: "io.example.orders@v1".to_string(),
                operations: vec!["fulfill".to_string()],
                parameter_constraints: None,
                result_constraints: None,
            }],
            constraints: ContractConstraints::default(),
            data_policy: DataPolicy::default(),
            obligations: vec![],
            commercial_terms: None,
            validity: ContractValidity {
                valid_from: now - chrono::Duration::hours(1),
                valid_until: now + chrono::Duration::hours(24),
            },
            protocol: ProtocolBinding::default(),
            evidence: ContractEvidence::default(),
            parent_contract_id: None,
            previous_contract_hash: None,
            created_at: now,
            updated_at: now,
        };

        // Only issuer signs
        let att1 = create_attestation(&contract, issuer_did, &issuer_key).unwrap();
        contract.evidence.attestations = vec![att1];

        let result = execute_activation_ceremony(contract, &party_keys);
        assert!(matches!(result, Err(ContractError::MissingAttestation(..))));
    }
}
