use crate::canonical::canonicalize_contract;
use crate::error::ContractError;
use crate::model::InteractionContract;
use sha2::{Digest, Sha256};

/// Computes the SHA-256 fingerprint (`sha256:<hex>`) of the canonical signable contract payload.
pub fn compute_contract_hash(contract: &InteractionContract) -> Result<String, ContractError> {
    let canonical = canonicalize_contract(contract)?;
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let hex = format!("{:x}", hasher.finalize());
    Ok(format!("sha256:{hex}"))
}

/// Verifies that a contract matches a specific hash.
pub fn verify_contract_hash(
    contract: &InteractionContract,
    expected_hash: &str,
) -> Result<(), ContractError> {
    let actual_hash = compute_contract_hash(contract)?;
    if actual_hash == expected_hash {
        Ok(())
    } else {
        Err(ContractError::ContractHashMismatch {
            expected: expected_hash.to_string(),
            actual: actual_hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use chrono::TimeZone;

    fn sample_contract() -> InteractionContract {
        InteractionContract {
            contract_id: "ctr_200".to_string(),
            version: 1,
            state: crate::state_machine::ContractState::Proposed,
            issuer: PartyIdentity::new_did("did:web:company-a.com"),
            counterparty: PartyIdentity::new_did("did:web:company-b.com"),
            purpose: Purpose {
                code: "logistics_coordination".to_string(),
                description: "Cross-border logistics dispatch".to_string(),
            },
            capabilities: vec![ContractCapability {
                capability_id: "io.logistics.quote@v1".to_string(),
                operations: vec!["request_quote".to_string()],
                parameter_constraints: None,
                result_constraints: None,
            }],
            constraints: ContractConstraints::default(),
            data_policy: DataPolicy::default(),
            obligations: vec![],
            commercial_terms: None,
            validity: ContractValidity {
                valid_from: chrono::Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap(),
                valid_until: chrono::Utc.with_ymd_and_hms(2026, 8, 31, 0, 0, 0).unwrap(),
            },
            protocol: ProtocolBinding::default(),
            evidence: ContractEvidence::default(),
            parent_contract_id: None,
            previous_contract_hash: None,
            created_at: chrono::Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap(),
            updated_at: chrono::Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap(),
        }
    }

    #[test]
    fn test_hash_computation_and_verification() {
        let contract = sample_contract();
        let hash = compute_contract_hash(&contract).unwrap();
        assert!(hash.starts_with("sha256:"));
        assert!(verify_contract_hash(&contract, &hash).is_ok());

        assert!(verify_contract_hash(&contract, "sha256:invalidhash").is_err());
    }

    #[test]
    fn test_hash_changes_on_term_modification() {
        let mut contract = sample_contract();
        let hash1 = compute_contract_hash(&contract).unwrap();

        contract.constraints.max_transaction_value = Some(ContractMoney {
            amount_minor: 100000,
            currency: "USD".to_string(),
        });
        let hash2 = compute_contract_hash(&contract).unwrap();

        assert_ne!(hash1, hash2);
    }
}
