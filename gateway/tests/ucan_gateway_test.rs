use axum::http::StatusCode;
use axum::response::Json;
use gateway::ucan_api::{
    create_identity_handler, create_proof_handler, mint_ucan_handler, validate_ucan_handler,
    verify_proof_handler, CreateProofRequest, MintUcanRequest, ValidateUcanRequest,
    VerifyProofRequest,
};
use ssi_crypto::did::create_did_twin;
use ssi_crypto::ucan::{self, Capability, UcanToken};

#[tokio::test]
async fn test_ucan_identity_and_mint_endpoints() {
    // 1. Create User Identity
    let (status_user, Json(user_id)) = create_identity_handler().await;
    assert_eq!(status_user, StatusCode::OK);
    assert!(user_id.did.starts_with("did:twin:z"));
    assert_eq!(user_id.signing_seed_hex.len(), 64);

    // 2. Create Agent Identity
    let (status_agent, Json(agent_id)) = create_identity_handler().await;
    assert_eq!(status_agent, StatusCode::OK);

    // 3. Mint UCAN delegation token
    let mint_req = MintUcanRequest {
        issuer: user_id.did.clone(),
        audience: agent_id.did.clone(),
        capabilities: vec![Capability {
            resource: "google_calendar".to_string(),
            action: "create_event".to_string(),
        }],
        ttl_seconds: 600,
        issuer_seed_hex: user_id.signing_seed_hex,
    };

    let (status_mint, Json(mint_resp)) = mint_ucan_handler(Json(mint_req)).await;
    assert_eq!(status_mint, StatusCode::OK);
    assert!(mint_resp.success);
    let ucan_token = mint_resp.ucan_token.expect("Must return ucan_token");

    // 4. Validate the minted token
    let val_req = ValidateUcanRequest {
        ucan_token,
        required_resource: "google_calendar".to_string(),
        required_action: "create_event".to_string(),
        now_epoch: None,
    };
    let (status_val, Json(val_resp)) = validate_ucan_handler(Json(val_req)).await;
    assert_eq!(status_val, StatusCode::OK);
    assert!(val_resp.authorized);
    assert_eq!(val_resp.issuer, Some(user_id.did));
    assert_eq!(val_resp.audience, Some(agent_id.did));
}

#[tokio::test]
async fn test_ucan_validate_endpoint_authorized() {
    let user_identity = create_did_twin();
    let agent_identity = create_did_twin();

    let mut token = UcanToken {
        issuer: user_identity.did.clone(),
        audience: agent_identity.did.clone(),
        capabilities: vec![
            Capability {
                resource: "google_calendar".to_string(),
                action: "create_event".to_string(),
            },
            Capability {
                resource: "billing/refunds".to_string(),
                action: "execute".to_string(),
            },
        ],
        expiry: 9999999999,
        proof_chain: vec![],
        token_id: "ucan-test-1".to_string(),
        signature: None,
    };
    ucan::sign_ucan(&mut token, &user_identity.signing_seed).unwrap();
    let ucan_json = ucan::encode_ucan(&token).unwrap();

    let req = ValidateUcanRequest {
        ucan_token: ucan_json,
        required_resource: "google_calendar".to_string(),
        required_action: "create_event".to_string(),
        now_epoch: Some(1000),
    };

    let (status, Json(resp)) = validate_ucan_handler(Json(req)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(resp.status, "Authorized");
    assert!(resp.authorized);
    assert_eq!(resp.issuer, Some(user_identity.did));
    assert_eq!(resp.audience, Some(agent_identity.did));
}

#[tokio::test]
async fn test_ucan_validate_endpoint_unauthorized_action() {
    let user_identity = create_did_twin();
    let agent_identity = create_did_twin();

    let mut token = UcanToken {
        issuer: user_identity.did.clone(),
        audience: agent_identity.did.clone(),
        capabilities: vec![Capability {
            resource: "google_calendar".to_string(),
            action: "read".to_string(),
        }],
        expiry: 9999999999,
        proof_chain: vec![],
        token_id: "ucan-test-2".to_string(),
        signature: None,
    };
    ucan::sign_ucan(&mut token, &user_identity.signing_seed).unwrap();
    let ucan_json = ucan::encode_ucan(&token).unwrap();

    let req = ValidateUcanRequest {
        ucan_token: ucan_json,
        required_resource: "stripe".to_string(),
        required_action: "refund".to_string(),
        now_epoch: Some(1000),
    };

    let (status, Json(resp)) = validate_ucan_handler(Json(req)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(resp.status, "RequiresApproval");
    assert!(!resp.authorized);
}

#[tokio::test]
async fn test_ucan_selective_disclosure_proof_flow() {
    let user_identity = create_did_twin();
    let agent_identity = create_did_twin();

    let mut token = UcanToken {
        issuer: user_identity.did.clone(),
        audience: agent_identity.did.clone(),
        capabilities: vec![
            Capability {
                resource: "google_calendar".to_string(),
                action: "create_event".to_string(),
            },
            Capability {
                resource: "secret_salary_db".to_string(),
                action: "read".to_string(),
            },
        ],
        expiry: 9999999999,
        proof_chain: vec![],
        token_id: "ucan-proof-test".to_string(),
        signature: None,
    };
    ucan::sign_ucan(&mut token, &user_identity.signing_seed).unwrap();
    let ucan_json = ucan::encode_ucan(&token).unwrap();

    // 1. Generate selective disclosure proof for ONLY google_calendar:create_event
    let create_req = CreateProofRequest {
        ucan_token: ucan_json,
        required_resource: "google_calendar".to_string(),
        required_action: "create_event".to_string(),
        agent_seed_hex: hex::encode(agent_identity.signing_seed),
    };

    let (status, Json(create_resp)) = create_proof_handler(Json(create_req)).await;
    assert_eq!(status, StatusCode::OK);
    let proof = create_resp.proof.expect("Proof must be generated");

    // 2. Verify the proof
    let verify_req = VerifyProofRequest {
        proof,
        user_pubkey_hex: hex::encode(user_identity.public_key),
        agent_pubkey_hex: hex::encode(agent_identity.public_key),
    };

    let (status, Json(verify_resp)) = verify_proof_handler(Json(verify_req)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(verify_resp.valid);
}
