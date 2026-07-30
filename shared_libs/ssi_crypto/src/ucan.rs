//! UCAN (User Controlled Authorization Networks) types and validation.
//!
//! Core UCAN data structures for the Connection Model:
//! - `UcanToken` — a capability delegation token
//! - `Capability` — a resource + action pair
//! - `ActionRequest` / `ActionResponse` — the cryptographic leash exchange
//!
//! This is a minimal subset of the UCAN spec sufficient for B2B delegation.

use serde::{Deserialize, Serialize};

/// A single capability (resource + action) that can be delegated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    /// The resource being accessed (e.g., "messaging", "calendar", "payment")
    pub resource: String,
    /// The action permitted (e.g., "send", "read", "execute")
    pub action: String,
}

/// A UCAN delegation token.
///
/// Represents a user granting specific capabilities to a B2B agent.
/// The user's wallet mints these tokens during the connection onboarding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UcanToken {
    /// The DID of the entity granting the capabilities (user's pairwise DID)
    pub issuer: String,
    /// The DID of the entity receiving the capabilities (B2B service DID)
    pub audience: String,
    /// The specific capabilities being delegated
    pub capabilities: Vec<Capability>,
    /// Expiry timestamp (Unix epoch seconds). 0 = no expiry.
    pub expiry: u64,
    /// Chain of proof tokens (for delegation chains). Empty for root grants.
    pub proof_chain: Vec<String>,
    /// Unique token identifier
    pub token_id: String,
    /// Cryptographic signature of the token, signed by the issuer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Vec<u8>>,
}

/// An action request sent from the B2B agent to the user's wallet
/// when a tool requires explicit user approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequest {
    /// Unique request identifier
    pub request_id: String,
    /// The MCP tool that requires approval (e.g., "process_refund")
    pub tool_name: String,
    /// Human-readable summary (e.g., "Approve $42 refund")
    pub human_summary: String,
    /// SHA-256 hash of the exact tool arguments
    pub payload_hash: String,
    /// Expiry timestamp (Unix epoch seconds)
    pub expires_at: u64,
}

/// An action response returned from the user's wallet after approval/rejection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResponse {
    /// Must match the `request_id` from the corresponding `ActionRequest`
    pub request_id: String,
    /// Whether the user approved the action
    pub approved: bool,
    /// Ed25519 signature of the `payload_hash` (present only if approved)
    pub signature: Option<Vec<u8>>,
}

/// Result of UCAN validation.
#[derive(Debug, Clone)]
pub enum UcanValidationResult {
    /// The UCAN grants the required capability
    Authorized,
    /// The UCAN does not grant the required capability — needs wallet approval
    RequiresApproval,
    /// The UCAN is expired or otherwise invalid
    Denied(String),
}

/// Encode a UCAN token to a JSON string.
pub fn encode_ucan(token: &UcanToken) -> Result<String, String> {
    serde_json::to_string(token).map_err(|e| format!("UCAN encoding failed: {e}"))
}

/// Decode a UCAN token from a JSON string.
pub fn decode_ucan(json: &str) -> Result<UcanToken, String> {
    serde_json::from_str(json).map_err(|e| format!("UCAN decoding failed: {e}"))
}

/// Validate whether a UCAN token grants a specific capability.
///
/// # Arguments
/// * `token` - The UCAN token to validate
/// * `required_cap` - The capability needed for the current operation
/// * `now_epoch` - Current Unix timestamp for expiry checking
///
/// # Returns
/// `UcanValidationResult` indicating whether the capability is authorized,
/// requires wallet approval, or is denied.
pub fn validate_ucan(
    token: &UcanToken,
    required_cap: &Capability,
    now_epoch: u64,
) -> UcanValidationResult {
    // 1. Check expiry
    if token.expiry > 0 && now_epoch > token.expiry {
        return UcanValidationResult::Denied("UCAN token expired".to_string());
    }

    // 2. Check if the required capability is in the token's capability list
    for cap in &token.capabilities {
        if capability_satisfies(cap, required_cap) {
            return UcanValidationResult::Authorized;
        }
    }

    // 3. Capability not found — requires wallet approval
    UcanValidationResult::RequiresApproval
}

/// Check if a granted capability satisfies a required capability.
///
/// A capability satisfies the requirement if:
/// - Resources match exactly OR the granted resource is "*" (wildcard)
/// - Actions match exactly OR the granted action is "*" (wildcard)
fn capability_satisfies(granted: &Capability, required: &Capability) -> bool {
    let resource_match = granted.resource == "*" || granted.resource == required.resource;
    let action_match = granted.action == "*" || granted.action == required.action;
    resource_match && action_match
}

/// Create an ActionRequest with a SHA-256 hash of the tool arguments.
pub fn create_action_request(
    tool_name: &str,
    args_json: &str,
    human_summary: &str,
    ttl_seconds: u64,
    now_epoch: u64,
) -> ActionRequest {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(tool_name.as_bytes());
    hasher.update(b":");
    hasher.update(args_json.as_bytes());
    hasher.update(b":");
    hasher.update(now_epoch.to_be_bytes());
    let hash = hasher.finalize();

    ActionRequest {
        request_id: hex::encode(&hash[..16]), // 128-bit request ID
        tool_name: tool_name.to_string(),
        human_summary: human_summary.to_string(),
        payload_hash: hex::encode(hash),
        expires_at: now_epoch + ttl_seconds,
    }
}

/// Verify an ActionResponse signature against the original payload hash.
///
/// # Arguments
/// * `response` - The ActionResponse from the wallet
/// * `expected_hash` - The original `payload_hash` from the ActionRequest
/// * `user_pubkey` - The user's Ed25519 public key (from their pairwise DID)
pub fn verify_action_response(
    response: &ActionResponse,
    expected_hash: &str,
    user_pubkey: &[u8; 32],
) -> Result<bool, String> {
    if !response.approved {
        return Ok(false);
    }

    let sig_bytes = response
        .signature
        .as_ref()
        .ok_or("Missing signature in approved response")?;

    if sig_bytes.len() != 64 {
        return Err("Invalid signature length".to_string());
    }

    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(sig_bytes);

    let hash_bytes =
        hex::decode(expected_hash).map_err(|_| "Invalid payload hash hex".to_string())?;

    Ok(crate::signing::verify_bytes(
        user_pubkey,
        &hash_bytes,
        &sig_arr,
    ))
}

/// Sign a UCAN token using the issuer's private key.
pub fn sign_ucan(token: &mut UcanToken, issuer_seed: &[u8; 32]) -> Result<(), String> {
    let mut token_copy = token.clone();
    token_copy.signature = None;
    let serialized = serde_json::to_string(&token_copy).map_err(|e| e.to_string())?;
    let sig = crate::signing::sign_bytes(issuer_seed, serialized.as_bytes());
    token.signature = Some(sig.to_vec());
    Ok(())
}

/// Verify a UCAN token signature using the issuer's public key.
pub fn verify_ucan_signature(token: &UcanToken, issuer_pubkey: &[u8; 32]) -> Result<bool, String> {
    let sig_bytes = token
        .signature
        .as_ref()
        .ok_or("Missing signature in UCAN token")?;

    if sig_bytes.len() != 64 {
        return Err("Invalid signature length".to_string());
    }

    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(sig_bytes);

    let mut token_copy = token.clone();
    token_copy.signature = None;
    let serialized = serde_json::to_string(&token_copy).map_err(|e| e.to_string())?;

    Ok(crate::signing::verify_bytes(
        issuer_pubkey,
        serialized.as_bytes(),
        &sig_arr,
    ))
}

/// A standard Virtual Passport containing the agent's identity and its delegated UCANs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualPassport {
    /// The agent's own decentralized identity (did:twin:z...)
    pub agent_did: String,
    /// The verification public key of the agent
    pub agent_public_key: Vec<u8>,
    /// Active UCAN tokens delegating permissions from the user
    pub active_delegations: Vec<UcanToken>,
}

/// A cryptographically verifiable proof of a single capability delegation.
/// Used for secure selective disclosure to external verification entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashedCapabilityProof {
    /// SHA-256 hash of the specific capability (resource + action)
    pub capability_hash: String,
    /// The issuer's DID (user)
    pub issuer_did: String,
    /// The audience's DID (agent)
    pub audience_did: String,
    /// Token expiration timestamp
    pub expiry: u64,
    /// The specific UCAN token delegating this capability (includes the issuer signature)
    pub ucan_token: UcanToken,
    /// Ed25519 signature of the capability_hash + expiry, signed by the agent (audience) to prove ownership/presentation control
    pub agent_signature: Vec<u8>,
}

/// Generate a secure hashed capability proof for selective disclosure.
pub fn generate_hashed_proof(
    ucan_token: &UcanToken,
    target_cap: &Capability,
    agent_seed: &[u8; 32],
) -> Result<HashedCapabilityProof, String> {
    // Verify capability matches
    if !capability_satisfies(&ucan_token.capabilities[0], target_cap)
        && ucan_token
            .capabilities
            .iter()
            .all(|c| !capability_satisfies(c, target_cap))
    {
        return Err("UCAN does not grant the requested capability".to_string());
    }

    // Compute capability hash: sha256(resource:action)
    use sha2::{Digest, Sha256};
    let cap_str = format!("{}:{}", target_cap.resource, target_cap.action);
    let mut hasher = Sha256::new();
    hasher.update(cap_str.as_bytes());
    let hash = hasher.finalize();
    let cap_hash = hex::encode(hash);

    // Agent signs the presentation binding: cap_hash + expiry
    let mut sign_data = Vec::new();
    sign_data.extend_from_slice(cap_hash.as_bytes());
    sign_data.extend_from_slice(&ucan_token.expiry.to_be_bytes());
    let agent_signature = crate::signing::sign_bytes(agent_seed, &sign_data);

    Ok(HashedCapabilityProof {
        capability_hash: cap_hash,
        issuer_did: ucan_token.issuer.clone(),
        audience_did: ucan_token.audience.clone(),
        expiry: ucan_token.expiry,
        ucan_token: ucan_token.clone(),
        agent_signature: agent_signature.to_vec(),
    })
}

/// Verify a hashed capability proof peer-to-peer.
pub fn verify_hashed_proof(
    proof: &HashedCapabilityProof,
    user_pubkey: &[u8; 32],
    agent_pubkey: &[u8; 32],
) -> Result<bool, String> {
    // 1. Verify UCAN token signature (signed by User/Issuer)
    if !verify_ucan_signature(&proof.ucan_token, user_pubkey)? {
        return Ok(false);
    }

    // 2. Verify agent presentation signature
    if proof.agent_signature.len() != 64 {
        return Err("Invalid agent signature length".to_string());
    }
    let mut agent_sig_arr = [0u8; 64];
    agent_sig_arr.copy_from_slice(&proof.agent_signature);

    let mut sign_data = Vec::new();
    sign_data.extend_from_slice(proof.capability_hash.as_bytes());
    sign_data.extend_from_slice(&proof.expiry.to_be_bytes());

    if !crate::signing::verify_bytes(agent_pubkey, &sign_data, &agent_sig_arr) {
        return Ok(false);
    }

    // 3. Confirm the capability hashes to the claimed capability_hash
    use sha2::{Digest, Sha256};
    let mut found = false;
    for cap in &proof.ucan_token.capabilities {
        let cap_str = format!("{}:{}", cap.resource, cap.action);
        let mut hasher = Sha256::new();
        hasher.update(cap_str.as_bytes());
        let hash = hasher.finalize();
        if hex::encode(hash) == proof.capability_hash {
            found = true;
            break;
        }
    }

    Ok(found)
}

/// Compute a deterministic SHA-256 hash of a Virtual Passport.
///
/// The hash is computed as:
///   SHA-256( agent_did + ":" + sorted_capabilities.join(",") )
///
/// where each capability is formatted as "resource:action", sorted alphabetically.
/// This ensures the hash is order-independent and deterministic.
pub fn compute_passport_hash(passport: &VirtualPassport) -> String {
    use sha2::{Digest, Sha256};

    let mut all_caps: Vec<String> = passport
        .active_delegations
        .iter()
        .flat_map(|ucan| {
            ucan.capabilities
                .iter()
                .map(|c| format!("{}:{}", c.resource, c.action))
        })
        .collect();
    all_caps.sort();
    all_caps.dedup();

    let payload = format!("{}:{}", passport.agent_did, all_caps.join(","));
    let mut hasher = Sha256::new();
    hasher.update(payload.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_capability(resource: &str, action: &str) -> Capability {
        Capability {
            resource: resource.to_string(),
            action: action.to_string(),
        }
    }

    fn test_ucan() -> UcanToken {
        UcanToken {
            issuer: "did:twin:zuser123".to_string(),
            audience: "did:twin:zservice456".to_string(),
            capabilities: vec![
                test_capability("messaging", "send"),
                test_capability("calendar", "read"),
            ],
            expiry: 0, // no expiry
            proof_chain: vec![],
            token_id: "ucan-001".to_string(),
            signature: None,
        }
    }

    #[test]
    fn test_ucan_encode_decode_roundtrip() {
        let token = test_ucan();
        let json = encode_ucan(&token).unwrap();
        let decoded = decode_ucan(&json).unwrap();
        assert_eq!(decoded.issuer, token.issuer);
        assert_eq!(decoded.capabilities.len(), 2);
    }

    #[test]
    fn test_validate_ucan_authorized() {
        let token = test_ucan();
        let required = test_capability("messaging", "send");
        match validate_ucan(&token, &required, 100) {
            UcanValidationResult::Authorized => {}
            other => panic!("Expected Authorized, got {other:?}"),
        }
    }

    #[test]
    fn test_validate_ucan_requires_approval() {
        let token = test_ucan();
        let required = test_capability("payment", "transfer");
        match validate_ucan(&token, &required, 100) {
            UcanValidationResult::RequiresApproval => {}
            other => panic!("Expected RequiresApproval, got {other:?}"),
        }
    }

    #[test]
    fn test_validate_ucan_expired() {
        let mut token = test_ucan();
        token.expiry = 50;
        let required = test_capability("messaging", "send");
        match validate_ucan(&token, &required, 100) {
            UcanValidationResult::Denied(_) => {}
            other => panic!("Expected Denied, got {other:?}"),
        }
    }

    #[test]
    fn test_wildcard_capability() {
        let mut token = test_ucan();
        token.capabilities = vec![test_capability("*", "*")];
        let required = test_capability("payment", "transfer");
        match validate_ucan(&token, &required, 100) {
            UcanValidationResult::Authorized => {}
            other => panic!("Expected Authorized, got {other:?}"),
        }
    }

    #[test]
    fn test_create_action_request() {
        let req = create_action_request(
            "process_refund",
            r#"{"amount": 42}"#,
            "Approve $42 refund",
            300,
            1000,
        );
        assert_eq!(req.tool_name, "process_refund");
        assert_eq!(req.expires_at, 1300);
        assert!(!req.payload_hash.is_empty());
        assert!(!req.request_id.is_empty());
    }

    #[test]
    fn test_verify_action_response_with_signing() {
        use crate::did::create_did_twin;

        let identity = create_did_twin();
        let req = create_action_request("test_tool", "{}", "Test", 300, 1000);

        // User signs the payload_hash
        let hash_bytes = hex::decode(&req.payload_hash).unwrap();
        let signature = crate::signing::sign_bytes(&identity.signing_seed, &hash_bytes);

        let response = ActionResponse {
            request_id: req.request_id.clone(),
            approved: true,
            signature: Some(signature.to_vec()),
        };

        let result =
            verify_action_response(&response, &req.payload_hash, &identity.public_key).unwrap();
        assert!(result);
    }

    #[test]
    fn test_verify_action_response_rejected() {
        let identity = crate::did::create_did_twin();
        let response = ActionResponse {
            request_id: "test".to_string(),
            approved: false,
            signature: None,
        };

        let result = verify_action_response(&response, "abcd1234", &identity.public_key).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_ucan_signing_and_verification() {
        use crate::did::create_did_twin;
        let issuer_identity = create_did_twin();
        let mut token = test_ucan();

        sign_ucan(&mut token, &issuer_identity.signing_seed).unwrap();
        assert!(token.signature.is_some());

        let is_valid = verify_ucan_signature(&token, &issuer_identity.public_key).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_hashed_capability_proof_generation_and_verification() {
        use crate::did::create_did_twin;
        let user_identity = create_did_twin();
        let agent_identity = create_did_twin();

        let mut token = UcanToken {
            issuer: user_identity.did.clone(),
            audience: agent_identity.did.clone(),
            capabilities: vec![Capability {
                resource: "calendar".to_string(),
                action: "read".to_string(),
            }],
            expiry: 2000,
            proof_chain: vec![],
            token_id: "test-proof-ucan".to_string(),
            signature: None,
        };
        sign_ucan(&mut token, &user_identity.signing_seed).unwrap();

        let target_cap = Capability {
            resource: "calendar".to_string(),
            action: "read".to_string(),
        };

        let proof =
            generate_hashed_proof(&token, &target_cap, &agent_identity.signing_seed).unwrap();
        assert_eq!(proof.expiry, 2000);

        let verified = verify_hashed_proof(
            &proof,
            &user_identity.public_key,
            &agent_identity.public_key,
        )
        .unwrap();
        assert!(verified);
    }

    #[test]
    fn test_hashed_capability_proof_invalid_capability() {
        use crate::did::create_did_twin;
        let user_identity = create_did_twin();
        let agent_identity = create_did_twin();

        let mut token = UcanToken {
            issuer: user_identity.did.clone(),
            audience: agent_identity.did.clone(),
            capabilities: vec![Capability {
                resource: "calendar".to_string(),
                action: "read".to_string(),
            }],
            expiry: 2000,
            proof_chain: vec![],
            token_id: "test-proof-ucan".to_string(),
            signature: None,
        };
        sign_ucan(&mut token, &user_identity.signing_seed).unwrap();

        let invalid_cap = Capability {
            resource: "payment".to_string(),
            action: "execute".to_string(),
        };

        let res = generate_hashed_proof(&token, &invalid_cap, &agent_identity.signing_seed);
        assert!(res.is_err());
    }

    #[test]
    fn test_compute_passport_hash_determinism() {
        use crate::did::create_did_twin;
        let agent = create_did_twin();

        let cap_a = Capability {
            resource: "calendar".to_string(),
            action: "read".to_string(),
        };
        let cap_b = Capability {
            resource: "email".to_string(),
            action: "send".to_string(),
        };

        let passport_1 = VirtualPassport {
            agent_did: agent.did.clone(),
            agent_public_key: agent.public_key.to_vec(),
            active_delegations: vec![UcanToken {
                issuer: "did:twin:zUser1".to_string(),
                audience: agent.did.clone(),
                capabilities: vec![cap_a.clone(), cap_b.clone()],
                expiry: 9999,
                proof_chain: vec![],
                token_id: "ucan-1".to_string(),
                signature: None,
            }],
        };

        // Same caps in reversed order → same hash
        let passport_2 = VirtualPassport {
            agent_did: agent.did.clone(),
            agent_public_key: agent.public_key.to_vec(),
            active_delegations: vec![UcanToken {
                issuer: "did:twin:zUser1".to_string(),
                audience: agent.did.clone(),
                capabilities: vec![cap_b.clone(), cap_a.clone()],
                expiry: 9999,
                proof_chain: vec![],
                token_id: "ucan-2".to_string(),
                signature: None,
            }],
        };

        let hash_1 = compute_passport_hash(&passport_1);
        let hash_2 = compute_passport_hash(&passport_2);
        assert_eq!(hash_1, hash_2, "Passport hash must be order-independent");
        assert_eq!(hash_1.len(), 64, "SHA-256 hex should be 64 chars");

        // Different agent DID → different hash
        let other_agent = create_did_twin();
        let passport_3 = VirtualPassport {
            agent_did: other_agent.did.clone(),
            agent_public_key: other_agent.public_key.to_vec(),
            active_delegations: passport_1.active_delegations.clone(),
        };
        let hash_3 = compute_passport_hash(&passport_3);
        assert_ne!(
            hash_1, hash_3,
            "Different agent DID must produce different hash"
        );
    }
}
