// ─────────────────────────────────────────────────────────────
// OAuth Token — Canonical token types and broker trait.
//
// Consolidates the previously duplicated OAuthToken/TokenStore
// definitions from executor_host and connector_mcp_server into
// a single shared module.
//
// The `TokenBroker` trait enables transparent token lifecycle
// management (get, store, refresh, revoke) with auto-refresh
// capabilities. Community edition uses NATS KV; Professional
// edition can override with cloud KMS-backed implementations.
// ─────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};

/// OAuth token record for a tenant + provider.
///
/// Stored in the `oauth_tokens` NATS KV bucket.
/// Key format: `{tenant_id}_{provider}` (RULE 020: `_` separator).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub tenant_id: String,
    pub provider: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Unix timestamp when the access token expires.
    pub expires_at: i64,
    /// Granted OAuth scopes.
    pub scopes: Vec<String>,
    /// Unix timestamp when the token was first stored.
    pub created_at: i64,
}

impl OAuthToken {
    /// Check if the access token is still valid (not expired).
    ///
    /// Applies a 30-second buffer to avoid edge-case expiry
    /// during request flight time.
    pub fn is_valid(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        self.expires_at > (now + 30)
    }

    /// Check if the token has a refresh token available.
    pub fn can_refresh(&self) -> bool {
        self.refresh_token
            .as_ref()
            .map(|rt| !rt.is_empty())
            .unwrap_or(false)
    }

    /// Generate the NATS KV key for this token.
    /// RULE 020: Uses `_` as separator, never `:`.
    pub fn kv_key(&self) -> String {
        format!("{}_{}", self.tenant_id.replace('.', "_"), self.provider)
    }

    /// Generate a KV key from tenant_id and provider strings.
    pub fn make_kv_key(tenant_id: &str, provider: &str) -> String {
        format!("{}_{}", tenant_id.replace('.', "_"), provider)
    }
}

/// Errors from the TokenBroker.
#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("provider '{provider}' not connected for tenant '{tenant_id}'")]
    NotConnected { tenant_id: String, provider: String },
    #[error("access token expired and no refresh token available")]
    ExpiredNoRefresh,
    #[error("token refresh failed: {0}")]
    RefreshFailed(String),
    #[error("token store error: {0}")]
    StoreError(String),
}

/// OAuth provider configuration — describes how to authenticate
/// with a third-party OAuth2 provider.
///
/// Loaded from `oauth_providers.toml` at startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProviderConfig {
    /// Unique provider identifier (e.g., "google", "stripe", "hubspot").
    pub id: String,
    /// Human-readable display name (e.g., "Google Workspace").
    pub display_name: String,
    /// Authorization endpoint URL.
    pub auth_url: String,
    /// Token exchange endpoint URL.
    pub token_url: String,
    /// Default scopes to request.
    pub scopes: Vec<String>,
    /// Environment variable name for the client ID.
    ///
    /// Uses the "Environment-Variable-First" pattern from the hard rules.
    pub client_id_env: String,
    /// Environment variable name for the client secret.
    pub client_secret_env: String,
}

/// Unified OAuth2 token lifecycle broker.
///
/// Abstracts token retrieval, storage, refresh, and revocation.
///
/// - **Community**: `NatsTokenBroker` — NATS KV-backed with inline refresh.
/// - **Professional**: Can be overridden with cloud KMS-backed implementation
///   via the same dependency injection pattern as `ToolListingOverlay`.
#[async_trait::async_trait]
pub trait TokenBroker: Send + Sync {
    /// Retrieve a valid token for the given tenant and provider.
    ///
    /// If the stored token is expired but has a refresh token,
    /// the broker transparently refreshes it before returning.
    async fn get_valid_token(
        &self,
        tenant_id: &str,
        provider: &str,
    ) -> Result<OAuthToken, TokenError>;

    /// Store or update a token in the backing store.
    async fn store_token(&self, token: &OAuthToken) -> Result<(), TokenError>;

    /// Explicitly trigger a token refresh for the given tenant/provider.
    ///
    /// Uses the stored refresh_token to obtain new credentials from
    /// the provider's token endpoint. Performs an atomic KV swap
    /// to prevent concurrent refresh races.
    async fn refresh_token(
        &self,
        tenant_id: &str,
        provider: &str,
    ) -> Result<OAuthToken, TokenError>;

    /// Revoke and delete a stored token.
    async fn revoke_token(&self, tenant_id: &str, provider: &str) -> Result<(), TokenError>;

    /// List all connected providers for a tenant.
    async fn list_connected_providers(&self, tenant_id: &str) -> Result<Vec<String>, TokenError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oauth_token_kv_key_uses_underscore() {
        // RULE 020: JetStream composite keys use _ separator
        let token = OAuthToken {
            tenant_id: "org.example".to_string(),
            provider: "google".to_string(),
            access_token: "tok".to_string(),
            refresh_token: None,
            expires_at: 0,
            scopes: vec![],
            created_at: 0,
        };
        let key = token.kv_key();
        assert!(!key.contains(':'));
        assert!(!key.contains('.'));
        assert_eq!(key, "org_example_google");
    }

    #[test]
    fn make_kv_key_static() {
        let key = OAuthToken::make_kv_key("tenant.one", "stripe");
        assert_eq!(key, "tenant_one_stripe");
    }

    #[test]
    fn token_validity_check() {
        let now = chrono::Utc::now().timestamp();
        let valid = OAuthToken {
            tenant_id: "t".into(),
            provider: "p".into(),
            access_token: "a".into(),
            refresh_token: Some("r".into()),
            expires_at: now + 3600,
            scopes: vec![],
            created_at: now,
        };
        assert!(valid.is_valid());
        assert!(valid.can_refresh());

        let expired = OAuthToken {
            expires_at: now - 100,
            refresh_token: None,
            ..valid
        };
        assert!(!expired.is_valid());
        assert!(!expired.can_refresh());
    }

    #[test]
    fn oauth_token_round_trip() {
        let token = OAuthToken {
            tenant_id: "t1".into(),
            provider: "google".into(),
            access_token: "access".into(),
            refresh_token: Some("refresh".into()),
            expires_at: 1700000000,
            scopes: vec!["calendar:read".into()],
            created_at: 1699999000,
        };
        let json = serde_json::to_string(&token).unwrap();
        let restored: OAuthToken = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tenant_id, "t1");
        assert_eq!(restored.provider, "google");
        assert_eq!(restored.scopes, vec!["calendar:read"]);
    }

    #[test]
    fn provider_config_round_trip() {
        let config = OAuthProviderConfig {
            id: "google".into(),
            display_name: "Google Workspace".into(),
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".into(),
            token_url: "https://oauth2.googleapis.com/token".into(),
            scopes: vec!["calendar.events".into()],
            client_id_env: "GOOGLE_CLIENT_ID".into(),
            client_secret_env: "GOOGLE_CLIENT_SECRET".into(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let restored: OAuthProviderConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "google");
        assert_eq!(restored.client_id_env, "GOOGLE_CLIENT_ID");
    }
}
