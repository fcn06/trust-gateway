use crate::error::ContractError;
use crate::model::{AuthorityReference, ContractCapability, InteractionContract};
use chrono::Utc;

/// Verifies whether an authority reference (UCAN, VP, or WebAuthn assertion) validly delegates
/// the necessary capabilities to the contract party.
pub fn verify_delegated_authority(
    authority: &AuthorityReference,
    party_did: &str,
    required_capabilities: &[ContractCapability],
) -> Result<(), ContractError> {
    // 1. Verify subject matches party DID if specified
    if let Some(ref subject) = authority.subject_did {
        if subject != party_did {
            return Err(ContractError::InvalidDelegation(format!(
                "Authority subject mismatch: expected '{party_did}', found '{subject}'"
            )));
        }
    }

    // 2. Check expiration if present
    if let Some(exp) = authority.expires_at {
        if Utc::now() > exp {
            let ref_id = authority.uri.as_deref().unwrap_or(&authority.hash);
            return Err(ContractError::InvalidDelegation(format!(
                "Authority reference '{}' expired at {}",
                ref_id,
                exp.to_rfc3339()
            )));
        }
    }

    // 3. Check capability coverage (each required capability must be covered by authority capabilities)
    for cap in required_capabilities {
        let is_covered = authority.capabilities.iter().any(|granted_cap| {
            if granted_cap == "*" {
                return true;
            }
            if granted_cap == &cap.capability_id {
                return true;
            }
            // Prefix wildcards (e.g., "io.company.*")
            if let Some(prefix) = granted_cap.strip_suffix(".*") {
                if cap.capability_id.starts_with(prefix) {
                    return true;
                }
            }
            false
        });

        if !is_covered {
            let ref_id = authority.uri.as_deref().unwrap_or(&authority.hash);
            return Err(ContractError::InvalidDelegation(format!(
                "Required capability '{}' not covered by authority '{}'",
                cap.capability_id, ref_id
            )));
        }
    }

    Ok(())
}

/// Verifies that all required authorities declared in the contract are valid and current.
pub fn verify_contract_authorities(contract: &InteractionContract) -> Result<(), ContractError> {
    for authority in &contract.evidence.authority_evidence {
        let party_did = if let Some(ref subj) = authority.subject_did {
            if subj == &contract.issuer.did {
                &contract.issuer.did
            } else if subj == &contract.counterparty.did {
                &contract.counterparty.did
            } else {
                return Err(ContractError::InvalidDelegation(format!(
                    "Authority subject '{subj}' is neither contract issuer nor counterparty"
                )));
            }
        } else {
            &contract.counterparty.did
        };

        verify_delegated_authority(authority, party_did, &contract.capabilities)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use chrono::Duration;

    #[test]
    fn test_valid_authority_delegation() {
        let now = Utc::now();
        let auth = AuthorityReference {
            auth_type: "ucan".to_string(),
            hash: "sha256:bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi".to_string(),
            uri: Some(
                "urn:cid:bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi".to_string(),
            ),
            subject_did: Some("did:web:branch-agent.com".to_string()),
            capabilities: vec!["io.procurement.orders@v1".to_string()],
            expires_at: Some(now + Duration::hours(24)),
        };

        let caps = vec![ContractCapability {
            capability_id: "io.procurement.orders@v1".to_string(),
            operations: vec!["create".to_string()],
            parameter_constraints: None,
            result_constraints: None,
        }];

        assert!(verify_delegated_authority(&auth, "did:web:branch-agent.com", &caps).is_ok());
    }

    #[test]
    fn test_expired_authority_rejected() {
        let now = Utc::now();
        let auth = AuthorityReference {
            auth_type: "ucan".to_string(),
            hash: "sha256:expired".to_string(),
            uri: Some("urn:cid:bafy_expired".to_string()),
            subject_did: Some("did:web:branch-agent.com".to_string()),
            capabilities: vec!["*".to_string()],
            expires_at: Some(now - Duration::hours(1)),
        };

        let caps = vec![ContractCapability {
            capability_id: "io.procurement.orders@v1".to_string(),
            operations: vec!["create".to_string()],
            parameter_constraints: None,
            result_constraints: None,
        }];

        let res = verify_delegated_authority(&auth, "did:web:branch-agent.com", &caps);
        assert!(matches!(res, Err(ContractError::InvalidDelegation(..))));
    }

    #[test]
    fn test_uncovered_capability_rejected() {
        let auth = AuthorityReference {
            auth_type: "ucan".to_string(),
            hash: "sha256:billing".to_string(),
            uri: Some("urn:cid:bafy_billing".to_string()),
            subject_did: Some("did:web:branch-agent.com".to_string()),
            capabilities: vec!["io.billing.invoices@v1".to_string()],
            expires_at: None,
        };

        let caps = vec![ContractCapability {
            capability_id: "io.procurement.orders@v1".to_string(),
            operations: vec!["create".to_string()],
            parameter_constraints: None,
            result_constraints: None,
        }];

        let res = verify_delegated_authority(&auth, "did:web:branch-agent.com", &caps);
        assert!(matches!(res, Err(ContractError::InvalidDelegation(..))));
    }
}
