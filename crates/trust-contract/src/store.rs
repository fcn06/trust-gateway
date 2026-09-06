use crate::error::ContractError;
use crate::model::InteractionContract;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Formats a composite JetStream key strictly using underscore `_` per INV-009 / RULE[020_JETSTREAM_KEYS.md].
pub fn format_contract_key(tenant_id: &str, contract_id: &str) -> String {
    format!("{tenant_id}_{contract_id}")
}

/// Abstract contract store trait.
#[async_trait]
pub trait ContractStore: Send + Sync {
    /// Persist or update a contract.
    async fn put_contract(
        &self,
        tenant_id: &str,
        contract: InteractionContract,
    ) -> Result<(), ContractError>;

    /// Retrieve a contract by tenant_id and contract_id.
    async fn get_contract(
        &self,
        tenant_id: &str,
        contract_id: &str,
    ) -> Result<Option<InteractionContract>, ContractError>;

    /// List all contracts for a tenant.
    async fn list_contracts(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<InteractionContract>, ContractError>;

    /// Delete a contract by tenant_id and contract_id.
    async fn delete_contract(
        &self,
        tenant_id: &str,
        contract_id: &str,
    ) -> Result<(), ContractError>;
}

/// In-memory thread-safe implementation of ContractStore for testing and embedded usage.
#[derive(Debug, Default, Clone)]
pub struct InMemoryContractStore {
    contracts: Arc<RwLock<HashMap<String, InteractionContract>>>,
}

impl InMemoryContractStore {
    pub fn new() -> Self {
        Self {
            contracts: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ContractStore for InMemoryContractStore {
    async fn put_contract(
        &self,
        tenant_id: &str,
        contract: InteractionContract,
    ) -> Result<(), ContractError> {
        let key = format_contract_key(tenant_id, &contract.contract_id);
        let mut map = self.contracts.write().await;
        map.insert(key, contract);
        Ok(())
    }

    async fn get_contract(
        &self,
        tenant_id: &str,
        contract_id: &str,
    ) -> Result<Option<InteractionContract>, ContractError> {
        let key = format_contract_key(tenant_id, contract_id);
        let map = self.contracts.read().await;
        Ok(map.get(&key).cloned())
    }

    async fn list_contracts(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<InteractionContract>, ContractError> {
        let prefix = format!("{tenant_id}_");
        let map = self.contracts.read().await;
        let list = map
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(_, v)| v.clone())
            .collect();
        Ok(list)
    }

    async fn delete_contract(
        &self,
        tenant_id: &str,
        contract_id: &str,
    ) -> Result<(), ContractError> {
        let key = format_contract_key(tenant_id, contract_id);
        let mut map = self.contracts.write().await;
        map.remove(&key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use chrono::TimeZone;

    #[tokio::test]
    async fn test_in_memory_store_lifecycle() {
        let store = InMemoryContractStore::new();
        let tenant_id = "tenant_alpha";
        let contract_id = "ctr_999";

        let contract = InteractionContract {
            contract_id: contract_id.to_string(),
            version: 1,
            state: crate::state_machine::ContractState::Active,
            issuer: PartyIdentity::new_did("did:web:tenant-a.com"),
            counterparty: PartyIdentity::new_did("did:web:tenant-b.com"),
            purpose: Purpose {
                code: "sync".to_string(),
                description: "Sync".to_string(),
            },
            capabilities: vec![],
            constraints: ContractConstraints::default(),
            data_policy: DataPolicy::default(),
            obligations: vec![],
            commercial_terms: None,
            validity: ContractValidity {
                valid_from: chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
                valid_until: chrono::Utc.with_ymd_and_hms(2026, 12, 31, 0, 0, 0).unwrap(),
            },
            protocol: ProtocolBinding::default(),
            evidence: ContractEvidence::default(),
            parent_contract_id: None,
            previous_contract_hash: None,
            created_at: chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            updated_at: chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        };

        store
            .put_contract(tenant_id, contract.clone())
            .await
            .unwrap();

        let retrieved = store.get_contract(tenant_id, contract_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().contract_id, contract_id);

        let list = store.list_contracts(tenant_id).await.unwrap();
        assert_eq!(list.len(), 1);

        store.delete_contract(tenant_id, contract_id).await.unwrap();
        let after_delete = store.get_contract(tenant_id, contract_id).await.unwrap();
        assert!(after_delete.is_none());
    }

    #[test]
    fn test_format_contract_key_uses_underscore() {
        let key = format_contract_key("tenant1", "contract2");
        assert_eq!(key, "tenant1_contract2");
        assert!(!key.contains(':'));
    }
}
