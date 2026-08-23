// ─────────────────────────────────────────────────────────────
// UCAN HTTP API — Native Capability & Delegation Validation
//
// Exposes dedicated endpoints on the Trust Gateway for:
// 1. POST /v1/ucan/validate     — Validate capability against UCAN token
// 2. POST /v1/ucan/proof        — Generate selective disclosure proof
// 3. POST /v1/ucan/verify-proof — Verify selective disclosure proof
// ─────────────────────────────────────────────────────────────

use axum::{http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use ssi_crypto::ucan::{self, Capability, HashedCapabilityProof, UcanToken, UcanValidationResult};

/// Request payload for `POST /v1/ucan/validate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateUcanRequest {
    /// Serialized UCAN JSON token or compact string.
    pub ucan_token: String,
    /// The required resource (e.g., "google_calendar", "billing/refunds", "*").
    pub required_resource: String,
    /// The required action (e.g., "create_event", "execute", "*").
    pub required_action: String,
    /// Optional timestamp override (seconds since UNIX epoch). Defaults to now.
    #[serde(default)]
    pub now_epoch: Option<u64>,
}

/// Response payload for `POST /v1/ucan/validate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateUcanResponse {
    /// "Authorized", "RequiresApproval", or "Denied".
    pub status: String,
    pub authorized: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<Capability>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Handler for `POST /v1/ucan/validate`.
pub async fn validate_ucan_handler(
    Json(req): Json<ValidateUcanRequest>,
) -> (StatusCode, Json<ValidateUcanResponse>) {
    // 1. Decode UCAN token
    let token: UcanToken = match ucan::decode_ucan(&req.ucan_token) {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ValidateUcanResponse {
                    status: "Denied".to_string(),
                    authorized: false,
                    issuer: None,
                    audience: None,
                    capabilities: None,
                    expires_at: None,
                    error: Some(format!("Invalid UCAN encoding: {e}")),
                }),
            );
        }
    };

    // 2. Verify signature if public key can be extracted from issuer DID
    if let Some(pubkey) = ssi_crypto::did::parse_did_twin_pubkey(&token.issuer) {
        if let Err(e) = ucan::verify_ucan_signature(&token, &pubkey) {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ValidateUcanResponse {
                    status: "Denied".to_string(),
                    authorized: false,
                    issuer: Some(token.issuer),
                    audience: Some(token.audience),
                    capabilities: Some(token.capabilities),
                    expires_at: Some(token.expiry),
                    error: Some(format!("UCAN signature verification failed: {e}")),
                }),
            );
        }
    }

    // 3. Validate capability
    let required_cap = Capability {
        resource: req.required_resource,
        action: req.required_action,
    };
    let now = req
        .now_epoch
        .unwrap_or_else(|| chrono::Utc::now().timestamp() as u64);

    match ucan::validate_ucan(&token, &required_cap, now) {
        UcanValidationResult::Authorized => (
            StatusCode::OK,
            Json(ValidateUcanResponse {
                status: "Authorized".to_string(),
                authorized: true,
                issuer: Some(token.issuer),
                audience: Some(token.audience),
                capabilities: Some(token.capabilities),
                expires_at: Some(token.expiry),
                error: None,
            }),
        ),
        UcanValidationResult::RequiresApproval => (
            StatusCode::OK,
            Json(ValidateUcanResponse {
                status: "RequiresApproval".to_string(),
                authorized: false,
                issuer: Some(token.issuer),
                audience: Some(token.audience),
                capabilities: Some(token.capabilities),
                expires_at: Some(token.expiry),
                error: Some(
                    "Action is not covered by delegated capabilities — wallet approval required"
                        .to_string(),
                ),
            }),
        ),
        UcanValidationResult::Denied(reason) => (
            StatusCode::FORBIDDEN,
            Json(ValidateUcanResponse {
                status: "Denied".to_string(),
                authorized: false,
                issuer: Some(token.issuer),
                audience: Some(token.audience),
                capabilities: Some(token.capabilities),
                expires_at: Some(token.expiry),
                error: Some(reason),
            }),
        ),
    }
}

/// Request payload for `POST /v1/ucan/proof`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProofRequest {
    pub ucan_token: String,
    pub required_resource: String,
    pub required_action: String,
    /// 32-byte Ed25519 signing seed of the presenting agent in Hex.
    pub agent_seed_hex: String,
}

/// Response payload for `POST /v1/ucan/proof`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProofResponse {
    pub proof: Option<HashedCapabilityProof>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Handler for `POST /v1/ucan/proof`.
pub async fn create_proof_handler(
    Json(req): Json<CreateProofRequest>,
) -> (StatusCode, Json<CreateProofResponse>) {
    let token = match ucan::decode_ucan(&req.ucan_token) {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(CreateProofResponse {
                    proof: None,
                    error: Some(format!("Invalid UCAN encoding: {e}")),
                }),
            );
        }
    };

    let seed_bytes = match hex::decode(&req.agent_seed_hex) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(CreateProofResponse {
                    proof: None,
                    error: Some("Invalid agent_seed_hex: must be 32 bytes hex".to_string()),
                }),
            );
        }
    };

    let target_cap = Capability {
        resource: req.required_resource,
        action: req.required_action,
    };

    match ucan::generate_hashed_proof(&token, &target_cap, &seed_bytes) {
        Ok(proof) => (
            StatusCode::OK,
            Json(CreateProofResponse {
                proof: Some(proof),
                error: None,
            }),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(CreateProofResponse {
                proof: None,
                error: Some(e),
            }),
        ),
    }
}

/// Request payload for `POST /v1/ucan/verify-proof`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyProofRequest {
    pub proof: HashedCapabilityProof,
    /// 32-byte Ed25519 public key of user/issuer in Hex.
    pub user_pubkey_hex: String,
    /// 32-byte Ed25519 public key of presenting agent in Hex.
    pub agent_pubkey_hex: String,
}

/// Response payload for `POST /v1/ucan/verify-proof`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyProofResponse {
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Handler for `POST /v1/ucan/verify-proof`.
pub async fn verify_proof_handler(
    Json(req): Json<VerifyProofRequest>,
) -> (StatusCode, Json<VerifyProofResponse>) {
    let user_pubkey = match hex::decode(&req.user_pubkey_hex) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(VerifyProofResponse {
                    valid: false,
                    error: Some("Invalid user_pubkey_hex: must be 32 bytes hex".to_string()),
                }),
            );
        }
    };

    let agent_pubkey = match hex::decode(&req.agent_pubkey_hex) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(VerifyProofResponse {
                    valid: false,
                    error: Some("Invalid agent_pubkey_hex: must be 32 bytes hex".to_string()),
                }),
            );
        }
    };

    match ucan::verify_hashed_proof(&req.proof, &user_pubkey, &agent_pubkey) {
        Ok(valid) => (
            StatusCode::OK,
            Json(VerifyProofResponse { valid, error: None }),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(VerifyProofResponse {
                valid: false,
                error: Some(e),
            }),
        ),
    }
}

/// Response payload for `POST /v1/ucan/identity`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIdentityResponse {
    pub did: String,
    pub public_key_hex: String,
    pub signing_seed_hex: String,
}

/// Handler for `POST /v1/ucan/identity` — Generates a new self-sovereign DID identity.
pub async fn create_identity_handler() -> (StatusCode, Json<CreateIdentityResponse>) {
    let identity = ssi_crypto::did::create_did_twin();
    (
        StatusCode::OK,
        Json(CreateIdentityResponse {
            did: identity.did,
            public_key_hex: hex::encode(identity.public_key),
            signing_seed_hex: hex::encode(identity.signing_seed),
        }),
    )
}

fn default_ttl() -> u64 {
    3600
}

/// Request payload for `POST /v1/ucan/mint`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintUcanRequest {
    /// Issuer DID (did:twin:z...)
    pub issuer: String,
    /// Audience DID (did:twin:z...)
    pub audience: String,
    /// Delegated capabilities
    pub capabilities: Vec<Capability>,
    /// Lifetime in seconds (defaults to 3600 / 1 hour)
    #[serde(default = "default_ttl")]
    pub ttl_seconds: u64,
    /// Hex-encoded Ed25519 signing seed of the issuer (32 bytes)
    pub issuer_seed_hex: String,
}

/// Response payload for `POST /v1/ucan/mint`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintUcanResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ucan_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<UcanToken>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Handler for `POST /v1/ucan/mint` — Mints and cryptographically signs a valid UCAN delegation token.
pub async fn mint_ucan_handler(
    Json(req): Json<MintUcanRequest>,
) -> (StatusCode, Json<MintUcanResponse>) {
    let seed = match hex::decode(&req.issuer_seed_hex) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(MintUcanResponse {
                    success: false,
                    ucan_token: None,
                    token: None,
                    expires_at: None,
                    error: Some("Invalid issuer_seed_hex: must be 32 bytes hex".to_string()),
                }),
            );
        }
    };

    let now = chrono::Utc::now().timestamp() as u64;
    let expiry = now + req.ttl_seconds;
    let token_id = format!("ucan_{}", uuid::Uuid::new_v4());

    let mut token = UcanToken {
        issuer: req.issuer,
        audience: req.audience,
        capabilities: req.capabilities,
        expiry,
        proof_chain: vec![],
        token_id,
        signature: None,
    };

    if let Err(e) = ucan::sign_ucan(&mut token, &seed) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(MintUcanResponse {
                success: false,
                ucan_token: None,
                token: None,
                expires_at: None,
                error: Some(format!("Failed to sign UCAN token: {e}")),
            }),
        );
    }

    let serialized = match ucan::encode_ucan(&token) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(MintUcanResponse {
                    success: false,
                    ucan_token: None,
                    token: None,
                    expires_at: None,
                    error: Some(format!("Failed to encode UCAN token: {e}")),
                }),
            );
        }
    };

    (
        StatusCode::OK,
        Json(MintUcanResponse {
            success: true,
            ucan_token: Some(serialized),
            token: Some(token),
            expires_at: Some(expiry),
            error: None,
        }),
    )
}
