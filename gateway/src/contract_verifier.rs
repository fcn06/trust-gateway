use async_trait::async_trait;
use std::sync::Arc;
use trust_contract::{
    validate_action_against_contract, verify_contract_hash, ActionEvaluationContext, ContractError,
    ContractMoney, ContractStore,
};

/// Verified contract context witness token.
/// Only constructible after successful deterministic validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedContractContext {
    pub contract_id: String,
    pub contract_hash: String,
    pub capability_id: String,
}

/// Verification interface for B2B Negotiated Interaction Contracts.
#[async_trait]
pub trait ContractVerifier: Send + Sync {
    async fn verify_action(
        &self,
        tenant_id: &str,
        requester_did: &str,
        action_name: &str,
        arguments: &serde_json::Value,
        contract_ctx_value: &serde_json::Value,
    ) -> Result<VerifiedContractContext, ContractError>;
}

/// Default implementation of ContractVerifier leveraging a ContractStore.
pub struct DefaultContractVerifier {
    store: Arc<dyn ContractStore>,
}

impl DefaultContractVerifier {
    pub fn new(store: Arc<dyn ContractStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl ContractVerifier for DefaultContractVerifier {
    async fn verify_action(
        &self,
        tenant_id: &str,
        requester_did: &str,
        action_name: &str,
        arguments: &serde_json::Value,
        contract_ctx_value: &serde_json::Value,
    ) -> Result<VerifiedContractContext, ContractError> {
        let contract_id = contract_ctx_value
            .get("contract_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ContractError::InvalidContractStructure {
                detail: "contract_context missing 'contract_id'".to_string(),
            })?;

        let contract_hash = contract_ctx_value
            .get("contract_hash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ContractError::InvalidContractStructure {
                detail: "contract_context missing 'contract_hash'".to_string(),
            })?;

        let capability_id = contract_ctx_value
            .get("capability_id")
            .and_then(|v| v.as_str())
            .unwrap_or(action_name);

        let contract = self
            .store
            .get_contract(tenant_id, contract_id)
            .await?
            .ok_or_else(|| ContractError::ContractNotFound(contract_id.to_string()))?;

        // 1. Verify contract hash
        verify_contract_hash(&contract, contract_hash)?;

        // 2. Extract monetary amount if present in arguments
        let amount_opt = if let Some(amount_val) = arguments.get("amount") {
            if let Some(cents) = amount_val.get("amount_cents").and_then(|v| v.as_u64()) {
                let curr = amount_val
                    .get("currency")
                    .and_then(|v| v.as_str())
                    .unwrap_or("EUR")
                    .to_string();
                Some(ContractMoney {
                    amount_minor: cents,
                    currency: curr,
                })
            } else if let Some(cents) = amount_val.get("amount_minor").and_then(|v| v.as_u64()) {
                let curr = amount_val
                    .get("currency")
                    .and_then(|v| v.as_str())
                    .unwrap_or("EUR")
                    .to_string();
                Some(ContractMoney {
                    amount_minor: cents,
                    currency: curr,
                })
            } else {
                None
            }
        } else {
            None
        };

        // 3. Extract units if present in arguments
        let units_opt = arguments
            .get("quantity")
            .or_else(|| arguments.get("units"))
            .and_then(|v| v.as_u64());

        // 4. Extract destination/country if present in arguments
        let geo_opt = arguments
            .get("destination_country")
            .or_else(|| arguments.get("country"))
            .or_else(|| arguments.get("region"))
            .and_then(|v| v.as_str());

        // Infer operation name from action_name
        let operation = action_name.split('.').next_back().unwrap_or(action_name);

        let eval_ctx = ActionEvaluationContext {
            requester_did,
            capability_id,
            operation,
            amount: amount_opt.as_ref(),
            units: units_opt,
            destination_country: geo_opt,
            evaluation_time: chrono::Utc::now(),
        };

        // 5. Evaluate action terms against active contract
        validate_action_against_contract(&contract, &eval_ctx)?;

        Ok(VerifiedContractContext {
            contract_id: contract_id.to_string(),
            contract_hash: contract_hash.to_string(),
            capability_id: capability_id.to_string(),
        })
    }
}
