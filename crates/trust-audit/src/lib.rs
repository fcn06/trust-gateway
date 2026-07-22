use serde::{Deserialize, Serialize};
use trust_canonical::canonical_hash;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: String,
    pub action_id: String,
    pub tenant_id: String,
    pub actor_id: String,
    pub action_type: String,
    pub prev_hash: String,
    pub curr_hash: String,
    pub timestamp: i64,
}

impl AuditEvent {
    pub fn new(
        event_id: &str,
        action_id: &str,
        tenant_id: &str,
        actor_id: &str,
        action_type: &str,
        prev_hash: &str,
        timestamp: i64,
    ) -> Self {
        let payload = serde_json::json!({
            "event_id": event_id,
            "action_id": action_id,
            "tenant_id": tenant_id,
            "actor_id": actor_id,
            "action_type": action_type,
            "prev_hash": prev_hash,
            "timestamp": timestamp,
        });
        let curr_hash = canonical_hash(&payload);

        Self {
            event_id: event_id.to_string(),
            action_id: action_id.to_string(),
            tenant_id: tenant_id.to_string(),
            actor_id: actor_id.to_string(),
            action_type: action_type.to_string(),
            prev_hash: prev_hash.to_string(),
            curr_hash,
            timestamp,
        }
    }
}
