use crate::error::ContractError;
use crate::model::{ContractMoney, InteractionContract};
use chrono::{DateTime, Utc};

/// Validates the structural integrity and invariants of a contract aggregate.
pub fn validate_contract_structure(contract: &InteractionContract) -> Result<(), ContractError> {
    if contract.contract_id.trim().is_empty() {
        return Err(ContractError::InvalidContractStructure {
            detail: "contract_id cannot be empty".to_string(),
        });
    }

    if contract.issuer.did.trim().is_empty() {
        return Err(ContractError::InvalidContractStructure {
            detail: "issuer.did cannot be empty".to_string(),
        });
    }

    if contract.counterparty.did.trim().is_empty() {
        return Err(ContractError::InvalidContractStructure {
            detail: "counterparty.did cannot be empty".to_string(),
        });
    }

    if contract.issuer.did == contract.counterparty.did {
        return Err(ContractError::InvalidContractStructure {
            detail: "issuer and counterparty cannot have the same DID".to_string(),
        });
    }

    if contract.purpose.code.trim().is_empty() {
        return Err(ContractError::InvalidContractStructure {
            detail: "purpose.code cannot be empty".to_string(),
        });
    }

    if contract.capabilities.is_empty() {
        return Err(ContractError::InvalidContractStructure {
            detail: "contract must declare at least one capability".to_string(),
        });
    }

    for cap in &contract.capabilities {
        if cap.capability_id.trim().is_empty() {
            return Err(ContractError::InvalidContractStructure {
                detail: "capability_id cannot be empty".to_string(),
            });
        }
        if cap.operations.is_empty() {
            return Err(ContractError::InvalidContractStructure {
                detail: format!(
                    "capability '{}' must declare at least one operation",
                    cap.capability_id
                ),
            });
        }
    }

    if contract.validity.valid_until <= contract.validity.valid_from {
        return Err(ContractError::InvalidContractStructure {
            detail: format!(
                "valid_until ({}) must be after valid_from ({})",
                contract.validity.valid_until, contract.validity.valid_from
            ),
        });
    }

    Ok(())
}

/// Parameters for evaluating a proposed action against an active contract.
#[derive(Debug, Clone)]
pub struct ActionEvaluationContext<'a> {
    pub requester_did: &'a str,
    pub capability_id: &'a str,
    pub operation: &'a str,
    pub amount: Option<&'a ContractMoney>,
    pub units: Option<u64>,
    pub destination_country: Option<&'a str>,
    pub evaluation_time: DateTime<Utc>,
}

/// Evaluates whether a proposed action satisfies the contract terms and invariants.
pub fn validate_action_against_contract(
    contract: &InteractionContract,
    ctx: &ActionEvaluationContext,
) -> Result<(), ContractError> {
    // 1. Contract state must be Active
    if !contract.state.is_active() {
        return Err(ContractError::ContractNotActive(
            contract.contract_id.clone(),
            contract.state,
        ));
    }

    // 2. Validity period check
    if !contract.validity.is_valid_at(ctx.evaluation_time) {
        return Err(ContractError::ContractExpired(
            contract.contract_id.clone(),
            contract.validity.valid_until.to_rfc3339(),
        ));
    }

    // 3. Counterparty match
    let is_party =
        contract.issuer.did == ctx.requester_did || contract.counterparty.did == ctx.requester_did;
    if !is_party {
        return Err(ContractError::CounterpartyMismatch {
            requester: ctx.requester_did.to_string(),
            contract_id: contract.contract_id.clone(),
        });
    }

    // 4. Capability and operation check
    let capability = contract
        .capabilities
        .iter()
        .find(|c| c.capability_id == ctx.capability_id)
        .ok_or_else(|| ContractError::CapabilityNotAllowed(ctx.capability_id.to_string()))?;

    if !capability.operations.iter().any(|op| op == ctx.operation) {
        return Err(ContractError::OperationNotAllowed {
            capability: ctx.capability_id.to_string(),
            operation: ctx.operation.to_string(),
        });
    }

    // 5. Monetary constraint check
    if let Some(req_amount) = ctx.amount {
        if let Some(max_limit) = &contract.constraints.max_transaction_value {
            if req_amount.currency.to_uppercase() != max_limit.currency.to_uppercase() {
                return Err(ContractError::ArgumentConstraintViolation {
                    detail: format!(
                        "Currency mismatch: requested {}, contract allows {}",
                        req_amount.currency, max_limit.currency
                    ),
                });
            }
            if req_amount.amount_minor > max_limit.amount_minor {
                return Err(ContractError::ArgumentConstraintViolation {
                    detail: format!(
                        "Amount {} minor units exceeds contract limit of {} minor units",
                        req_amount.amount_minor, max_limit.amount_minor
                    ),
                });
            }
        }
    }

    // 6. Unit count check
    if let Some(req_units) = ctx.units {
        if let Some(max_units) = contract.constraints.max_units {
            if req_units > max_units {
                return Err(ContractError::ArgumentConstraintViolation {
                    detail: format!(
                        "Requested units ({req_units}) exceeds contract limit ({max_units})"
                    ),
                });
            }
        }
    }

    // 7. Geography check
    if let Some(geo) = ctx.destination_country {
        if !contract.constraints.allowed_geographies.is_empty()
            && !contract
                .constraints
                .allowed_geographies
                .iter()
                .any(|g| g.eq_ignore_ascii_case(geo))
        {
            return Err(ContractError::ArgumentConstraintViolation {
                detail: format!("Geography '{geo}' not allowed by contract constraints"),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use chrono::TimeZone;

    fn sample_active_contract() -> InteractionContract {
        InteractionContract {
            contract_id: "ctr_order_01".to_string(),
            version: 1,
            state: crate::state_machine::ContractState::Active,
            issuer: PartyIdentity::new_did("did:web:buyer.com"),
            counterparty: PartyIdentity::new_did("did:web:seller.com"),
            purpose: Purpose {
                code: "procure_goods".to_string(),
                description: "Procurement".to_string(),
            },
            capabilities: vec![ContractCapability {
                capability_id: "io.company.orders@v1".to_string(),
                operations: vec!["create".to_string(), "status".to_string()],
                parameter_constraints: None,
                result_constraints: None,
            }],
            constraints: ContractConstraints {
                max_transaction_value: Some(ContractMoney {
                    amount_minor: 2500000, // €25,000.00
                    currency: "EUR".to_string(),
                }),
                allowed_geographies: vec!["EU".to_string(), "DE".to_string(), "FR".to_string()],
                max_units: Some(1000),
                cancellation_terms: None,
                custom_constraints: std::collections::BTreeMap::new(),
            },
            data_policy: DataPolicy::default(),
            obligations: vec![],
            commercial_terms: None,
            validity: ContractValidity {
                valid_from: chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
                valid_until: chrono::Utc
                    .with_ymd_and_hms(2026, 12, 31, 23, 59, 59)
                    .unwrap(),
            },
            protocol: ProtocolBinding::default(),
            evidence: ContractEvidence::default(),
            parent_contract_id: None,
            previous_contract_hash: None,
            created_at: chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            updated_at: chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        }
    }

    #[test]
    fn test_valid_action() {
        let contract = sample_active_contract();
        let amount = ContractMoney {
            amount_minor: 1800000, // €18,000.00
            currency: "EUR".to_string(),
        };
        let ctx = ActionEvaluationContext {
            requester_did: "did:web:buyer.com",
            capability_id: "io.company.orders@v1",
            operation: "create",
            amount: Some(&amount),
            units: Some(600),
            destination_country: Some("FR"),
            evaluation_time: chrono::Utc.with_ymd_and_hms(2026, 8, 23, 12, 0, 0).unwrap(),
        };

        assert!(validate_action_against_contract(&contract, &ctx).is_ok());
    }

    #[test]
    fn test_exceed_amount() {
        let contract = sample_active_contract();
        let amount = ContractMoney {
            amount_minor: 3000000, // €30,000.00
            currency: "EUR".to_string(),
        };
        let ctx = ActionEvaluationContext {
            requester_did: "did:web:buyer.com",
            capability_id: "io.company.orders@v1",
            operation: "create",
            amount: Some(&amount),
            units: Some(600),
            destination_country: Some("FR"),
            evaluation_time: chrono::Utc.with_ymd_and_hms(2026, 8, 23, 12, 0, 0).unwrap(),
        };

        assert!(matches!(
            validate_action_against_contract(&contract, &ctx),
            Err(ContractError::ArgumentConstraintViolation { .. })
        ));
    }

    #[test]
    fn test_unauthorized_party() {
        let contract = sample_active_contract();
        let ctx = ActionEvaluationContext {
            requester_did: "did:web:impostor.com",
            capability_id: "io.company.orders@v1",
            operation: "create",
            amount: None,
            units: None,
            destination_country: None,
            evaluation_time: chrono::Utc.with_ymd_and_hms(2026, 8, 23, 12, 0, 0).unwrap(),
        };

        assert!(matches!(
            validate_action_against_contract(&contract, &ctx),
            Err(ContractError::CounterpartyMismatch { .. })
        ));
    }
}
