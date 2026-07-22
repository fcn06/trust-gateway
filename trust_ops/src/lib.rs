use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Key Lifecycle & Attestation Posture Tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorPosture {
    pub executor_id: String,
    pub binary_digest: String,
    pub attestation_level: String,
    pub last_seen_epoch: i64,
    pub is_revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningKeyMetadata {
    pub kid: String,
    pub public_key_pem: String,
    pub created_at_epoch: i64,
    pub expires_at_epoch: i64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwkKey {
    pub kty: String,
    pub crv: String,
    pub x: String,
    pub kid: String,
    pub use_claim: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwksSet {
    pub keys: Vec<JwkKey>,
}

pub struct TrustOperationsPlane {
    executors: HashMap<String, ExecutorPosture>,
    keys: HashMap<String, SigningKeyMetadata>,
    revoked_grant_ids: Vec<String>,
}

impl TrustOperationsPlane {
    pub fn new() -> Self {
        Self {
            executors: HashMap::new(),
            keys: HashMap::new(),
            revoked_grant_ids: Vec::new(),
        }
    }

    pub fn add_signing_key(&mut self, key: SigningKeyMetadata) {
        self.keys.insert(key.kid.clone(), key);
    }

    pub fn get_key(&self, kid: &str) -> Option<&SigningKeyMetadata> {
        self.keys.get(kid)
    }

    pub fn generate_jwks(&self) -> JwksSet {
        let keys = self
            .keys
            .values()
            .map(|k| JwkKey {
                kty: "OKP".to_string(),
                crv: "Ed25519".to_string(),
                x: k.public_key_pem.clone(),
                kid: k.kid.clone(),
                use_claim: "sig".to_string(),
            })
            .collect();
        JwksSet { keys }
    }

    pub fn register_executor(&mut self, posture: ExecutorPosture) {
        self.executors.insert(posture.executor_id.clone(), posture);
    }

    pub fn is_executor_healthy(&self, executor_id: &str, min_attestation: &str) -> bool {
        if let Some(posture) = self.executors.get(executor_id) {
            if posture.is_revoked {
                return false;
            }
            return posture.attestation_level == min_attestation || min_attestation == "any";
        }
        false
    }

    pub fn revoke_grant(&mut self, grant_id: &str) {
        self.revoked_grant_ids.push(grant_id.to_string());
    }

    pub fn is_grant_revoked(&self, grant_id: &str) -> bool {
        self.revoked_grant_ids.contains(&grant_id.to_string())
    }
}
