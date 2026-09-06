use crate::error::ContractError;
use crate::hash::compute_contract_hash;
use crate::model::{ContractAttestation, InteractionContract};
use chrono::Utc;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

/// Signs a contract using an Ed25519 key and produces a ContractAttestation record.
pub fn create_attestation(
    contract: &InteractionContract,
    signer_did: impl Into<String>,
    signing_key: &SigningKey,
) -> Result<ContractAttestation, ContractError> {
    let signer_did = signer_did.into();
    let contract_hash = compute_contract_hash(contract)?;

    // We sign the contract hash directly (or canonical payload)
    let signature = signing_key.sign(contract_hash.as_bytes());
    let sig_hex = hex::encode(signature.to_bytes());

    Ok(ContractAttestation {
        contract_id: contract.contract_id.clone(),
        contract_hash,
        signer_did,
        signature_algorithm: "Ed25519".to_string(),
        signature: sig_hex,
        signed_at: Utc::now(),
    })
}

/// Verifies a single ContractAttestation against the contract and public verifying key.
pub fn verify_attestation(
    contract: &InteractionContract,
    attestation: &ContractAttestation,
    verifying_key: &VerifyingKey,
) -> Result<(), ContractError> {
    // 1. Verify contract ID matches
    if attestation.contract_id != contract.contract_id {
        return Err(ContractError::InvalidAttestation(format!(
            "Contract ID mismatch: expected '{}', found '{}'",
            contract.contract_id, attestation.contract_id
        )));
    }

    // 2. Verify contract hash matches
    let computed_hash = compute_contract_hash(contract)?;
    if attestation.contract_hash != computed_hash {
        return Err(ContractError::ContractHashMismatch {
            expected: computed_hash,
            actual: attestation.contract_hash.clone(),
        });
    }

    // 3. Verify signature algorithm
    if attestation.signature_algorithm != "Ed25519" {
        return Err(ContractError::InvalidAttestation(format!(
            "Unsupported signature algorithm '{}'",
            attestation.signature_algorithm
        )));
    }

    // 4. Verify cryptographic signature
    let sig_bytes = hex::decode(&attestation.signature).map_err(|e| {
        ContractError::InvalidAttestation(format!("Signature hex decoding failed: {e}"))
    })?;

    if sig_bytes.len() != 64 {
        return Err(ContractError::InvalidAttestation(
            "Signature must be exactly 64 bytes".to_string(),
        ));
    }

    let signature = Signature::from_slice(&sig_bytes).map_err(|e| {
        ContractError::InvalidAttestation(format!("Malformed Ed25519 signature: {e}"))
    })?;

    verifying_key
        .verify(attestation.contract_hash.as_bytes(), &signature)
        .map_err(|_| ContractError::InvalidAttestation(attestation.signer_did.clone()))
}

/// Helper: Extracts 32-byte Ed25519 public key from a DID if formatted with inline hex/key.
pub fn extract_pubkey_from_did(did: &str) -> Option<[u8; 32]> {
    if let Some(stripped) = did.strip_prefix("did:twin:z") {
        if stripped.len() >= 64 {
            if let Ok(bytes) = hex::decode(&stripped[..64]) {
                if let Ok(arr) = bytes.try_into() {
                    return Some(arr);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use chrono::TimeZone;

    fn sample_contract() -> InteractionContract {
        InteractionContract {
            contract_id: "ctr_attest_01".to_string(),
            version: 1,
            state: crate::state_machine::ContractState::Accepted,
            issuer: PartyIdentity::new_did("did:web:company-a.com"),
            counterparty: PartyIdentity::new_did("did:twin:company-b.com"),
            purpose: Purpose {
                code: "procurement".to_string(),
                description: "Procurement".to_string(),
            },
            capabilities: vec![ContractCapability {
                capability_id: "io.company.orders@v1".to_string(),
                operations: vec!["create".to_string()],
                parameter_constraints: None,
                result_constraints: None,
            }],
            constraints: ContractConstraints::default(),
            data_policy: DataPolicy::default(),
            obligations: vec![],
            commercial_terms: None,
            validity: ContractValidity {
                valid_from: chrono::Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap(),
                valid_until: chrono::Utc.with_ymd_and_hms(2026, 8, 31, 0, 0, 0).unwrap(),
            },
            protocol: ProtocolBinding::default(),
            evidence: ContractEvidence::default(),
            parent_contract_id: None,
            previous_contract_hash: None,
            created_at: chrono::Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap(),
            updated_at: chrono::Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap(),
        }
    }

    #[test]
    fn test_attestation_signing_and_verification() {
        let keypair_bytes: [u8; 32] = [42u8; 32];
        let signing_key = SigningKey::from_bytes(&keypair_bytes);
        let verifying_key = signing_key.verifying_key();

        let contract = sample_contract();
        let signer_did = "did:web:company-a.com";

        let attestation = create_attestation(&contract, signer_did, &signing_key).unwrap();
        assert_eq!(attestation.signer_did, signer_did);
        assert_eq!(attestation.signature_algorithm, "Ed25519");

        let verify_res = verify_attestation(&contract, &attestation, &verifying_key);
        assert!(verify_res.is_ok());

        // Tamper with contract
        let mut tampered = contract.clone();
        tampered.constraints.allowed_geographies = vec!["US".to_string()];
        let tamper_res = verify_attestation(&tampered, &attestation, &verifying_key);
        assert!(tamper_res.is_err());
    }
}
