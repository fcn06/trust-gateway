use sha2::Digest;
use trust_canonical::canonical_hash;
use trust_model::ExecutionGrant;


pub struct GrantIssuer;

impl GrantIssuer {
    pub fn create_grant(
        action_id: &str,
        tenant_id: &str,
        tool_name: &str,
        arguments: &serde_json::Value,
        issuer: &str,
        ttl_seconds: i64,
    ) -> ExecutionGrant {
        let input_hash = canonical_hash(arguments);
        let now = chrono::Utc::now().timestamp();
        let nonce = format!("{:x}", sha2::Sha256::digest(format!("{}:{}", action_id, now).as_bytes()));

        ExecutionGrant {
            grant_id: format!("grant_{}", action_id),
            action_id: action_id.to_string(),
            tenant_id: tenant_id.to_string(),
            tool_name: tool_name.to_string(),
            input_hash,
            issuer: issuer.to_string(),
            expires_at: now + ttl_seconds,
            nonce,
        }
    }
}
