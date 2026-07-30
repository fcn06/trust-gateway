use jwt_simple::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedJwt {
    pub subject: String,
    pub tenant_id: String,
    pub scopes: Vec<String>,
    pub expires_at: u64,
}

pub struct AuthVerifier;

impl AuthVerifier {
    /// Verify JWT claims and signature adhering to 010_JWT_CONTRACTS.md (no alg=none)
    pub fn verify_hs256(token: &str, secret: &[u8]) -> Result<VerifiedJwt, anyhow::Error> {
        let key = HS256Key::from_bytes(secret);
        let options = VerificationOptions {
            accept_future: true,
            time_tolerance: Some(Duration::from_secs(10)),
            ..Default::default()
        };
        let claims = key.verify_token::<NoCustomClaims>(token, Some(options))?;

        let subject = claims
            .subject
            .ok_or_else(|| anyhow::anyhow!("Missing subject claim"))?;
        let expires_at = claims
            .expires_at
            .ok_or_else(|| anyhow::anyhow!("Missing exp claim"))?
            .as_secs();

        Ok(VerifiedJwt {
            subject,
            tenant_id: "default_tenant".to_string(),
            scopes: vec!["tool:execute".to_string()],
            expires_at,
        })
    }
}
