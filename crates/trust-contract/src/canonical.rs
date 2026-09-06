use crate::error::ContractError;
use crate::model::{InteractionContract, SignableContractPayload};
use trust_canonical::canonical_json;

/// Converts a contract into its canonical JSON string representation.
/// The canonical payload includes only signable contract terms (excluding signatures, runtime state).
pub fn canonicalize_contract(contract: &InteractionContract) -> Result<String, ContractError> {
    let signable: SignableContractPayload = contract.into();
    let value = serde_json::to_value(&signable)
        .map_err(|e| ContractError::SerializationError(e.to_string()))?;
    Ok(canonical_json(&value))
}

/// Converts a signable payload directly into its canonical JSON string.
pub fn canonicalize_signable_payload(
    payload: &SignableContractPayload,
) -> Result<String, ContractError> {
    let value = serde_json::to_value(payload)
        .map_err(|e| ContractError::SerializationError(e.to_string()))?;
    Ok(canonical_json(&value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use chrono::TimeZone;

    #[test]
    fn test_canonical_determinism() {
        let contract1 = InteractionContract {
            contract_id: "ctr_101".to_string(),
            version: 1,
            state: crate::state_machine::ContractState::Active,
            issuer: PartyIdentity::new_did("did:web:company-a.com"),
            counterparty: PartyIdentity::new_did("did:twin:company-b.com"),
            purpose: Purpose {
                code: "supplier_order_management".to_string(),
                description: "Supplier order integration".to_string(),
            },
            capabilities: vec![ContractCapability {
                capability_id: "io.company.orders@v1".to_string(),
                operations: vec!["create".to_string(), "status".to_string()],
                parameter_constraints: None,
                result_constraints: None,
            }],
            constraints: ContractConstraints {
                max_transaction_value: Some(ContractMoney {
                    amount_minor: 5000000,
                    currency: "EUR".to_string(),
                }),
                allowed_geographies: vec!["EU".to_string()],
                max_units: Some(1000),
                cancellation_terms: Some("before_dispatch".to_string()),
                custom_constraints: std::collections::BTreeMap::new(),
            },
            data_policy: DataPolicy::default(),
            obligations: vec![],
            commercial_terms: None,
            validity: ContractValidity {
                valid_from: chrono::Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap(),
                valid_until: chrono::Utc
                    .with_ymd_and_hms(2026, 8, 31, 23, 59, 59)
                    .unwrap(),
            },
            protocol: ProtocolBinding::default(),
            evidence: ContractEvidence::default(),
            parent_contract_id: None,
            previous_contract_hash: None,
            created_at: chrono::Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap(),
            updated_at: chrono::Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap(),
        };

        let json1 = canonicalize_contract(&contract1).unwrap();
        let json2 = canonicalize_contract(&contract1).unwrap();
        assert_eq!(json1, json2);
    }
}
